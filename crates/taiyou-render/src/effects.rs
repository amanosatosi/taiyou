//! Scalar mask filters independently expressed from the pinterf filter equations.
//! Source locations, arithmetic choices and backend limits: docs/T018-CANDIDATE.md.
use std::sync::Arc;
use taiyou_types::*;

const GAUSSIAN_THRESHOLD: f64 = 0.333333_f32 as f64;

fn limit() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "blur support/pass/dimension limit",
    )
}
fn allocation() -> RenderError {
    RenderError::new(ErrorCode::AllocationFailed, "blur buffer allocation failed")
}
fn zeros<T: Default + Clone>(n: usize) -> Result<Vec<T>, RenderError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| allocation())?;
    v.resize(n, T::default());
    Ok(v)
}

/// Resolved target-space filter. Target scale is output/original-video size,
/// not text scale or SBAS. LayoutRes overrides the host's original video size;
/// without either, target scale is (1,1).
pub struct BlurSpec {
    gaussian: bool,
    sigma: [f64; 2],
    radius: [u32; 2],
    padding: [u32; 2],
    passes: u32,
    fractional: u32,
    active: bool,
}
impl BlurSpec {
    pub fn new(
        blur: FiniteF64,
        be: FiniteF64,
        target: [f64; 2],
        limits: &Limits,
    ) -> Result<Self, RenderError> {
        if target.iter().any(|s| !s.is_finite() || *s <= 0.0) || blur.get() < 0.0 {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "invalid blur target scale/strength",
            ));
        }
        // xy stores fBlur in a float. Negative strengths do not enable BE.
        let edge = be.get().max(0.0) as f32;
        let active = edge > 0.0 || blur.get() > GAUSSIAN_THRESHOLD;
        let gaussian = active && blur.get() > 0.0;
        let mut radius = [0; 2];
        let mut padding = [0; 2];
        let mut sigma = [0.0; 2];
        let mut passes = 0;
        let mut fractional = 0;
        if active {
            if !edge.is_finite() {
                return Err(limit());
            }
            for axis in 0..2 {
                if gaussian {
                    sigma[axis] = blur.get() * target[axis];
                    let width = (3.0 * sigma[axis]).ceil();
                    if !width.is_finite() || width > f64::from(limits.filter_radius()) * 2.0 + 1.0 {
                        return Err(limit());
                    }
                    radius[axis] = ((width as u32) | 1) / 2;
                }
                if edge > 0.0 {
                    let growth =
                        (f64::from(target[axis] as f32 * (edge * 0.25).sqrt()) + 0.5).floor();
                    if !growth.is_finite() || growth > f64::from(limits.filter_radius()) {
                        return Err(limit());
                    }
                    padding[axis] = growth as u32;
                }
                if radius[axis]
                    .checked_add(padding[axis])
                    .is_none_or(|n| n > limits.filter_radius())
                {
                    return Err(limit());
                }
            }
            let scaled = edge * 0.5 * (target[0] as f32 + target[1] as f32);
            if !scaled.is_finite() || scaled.ceil() > limits.be_passes() as f32 {
                return Err(limit());
            }
            passes = scaled as u32;
            let residual = scaled - passes as f32;
            if residual > 0.0 {
                fractional = (256.0 * residual / (1.0 + 3.0 * residual)) as u32;
            }
        }
        Ok(Self {
            gaussian,
            sigma,
            radius,
            padding,
            passes,
            fractional,
            active,
        })
    }
    pub const fn active(&self) -> bool {
        self.active
    }
    pub const fn support(&self) -> [u32; 2] {
        [
            self.radius[0] + self.padding[0],
            self.radius[1] + self.padding[1],
        ]
    }
}

// Float coefficients: double exponential samples stored as float before
// normalization by the double volume (as in GaussianCoefficients::init).
fn coefficients(sigma: f64, radius: u32) -> Result<Vec<f32>, RenderError> {
    let mut c = zeros::<f32>(2 * radius as usize + 1)?;
    let mid = radius as usize;
    c[mid] = 1.0;
    let mut volume = 1.0;
    if radius != 0 {
        let step = (-1.0 / (2.0 * sigma * sigma)).exp();
        let mut value = 1.0;
        let mut ratio = step;
        for i in 1..=mid {
            value *= ratio;
            ratio *= step * step;
            c[mid - i] = value as f32;
            c[mid + i] = value as f32;
            volume += value;
            volume += value;
        }
    }
    for v in &mut c {
        *v = (f64::from(*v) / volume) as f32;
    }
    Ok(c)
}

fn extent(
    mask: &Mask,
    halo: [u32; 2],
    limits: &Limits,
) -> Result<(PixelRect, usize, usize), RenderError> {
    let b = mask.bounds();
    let hx = i32::try_from(halo[0]).map_err(|_| limit())?;
    let hy = i32::try_from(halo[1]).map_err(|_| limit())?;
    let bounds = PixelRect::new(
        b.x0().checked_sub(hx).ok_or_else(limit)?,
        b.y0().checked_sub(hy).ok_or_else(limit)?,
        b.x1().checked_add(hx).ok_or_else(limit)?,
        b.y1().checked_add(hy).ok_or_else(limit)?,
    )
    .map_err(|_| limit())?;
    let n = u64::from(bounds.width())
        .checked_mul(u64::from(bounds.height()))
        .ok_or_else(limit)?;
    if bounds.width() > limits.max_dimension()
        || bounds.height() > limits.max_dimension()
        || n > limits.output_pixels()
        || n > limits.output_bytes()
    {
        return Err(limit());
    }
    // Two float work planes, byte input/output and Arc publication, coefficients.
    let work = n
        .checked_mul(12)
        .and_then(|n| n.checked_add(8 * (u64::from(halo[0]) + u64::from(halo[1]) + 1)))
        .ok_or_else(limit)?;
    if work > limits.transient_bytes() {
        return Err(RenderError::transient_limit(
            "blur scratch/halo reservation exceeded",
        ));
    }
    Ok((bounds, bounds.width() as usize, bounds.height() as usize))
}

/// Immutable full-support filtering, before viewport/clip composition.
pub fn apply_effects(
    mask: &Arc<Mask>,
    spec: &BlurSpec,
    limits: &Limits,
) -> Result<Arc<Mask>, RenderError> {
    if !spec.active || mask.bounds().is_empty() {
        return Ok(mask.clone());
    }
    // Preflight the entire combined operation before any filtering/allocation.
    extent(mask, spec.support(), limits)?;
    let mut result = mask.clone();
    if spec.gaussian {
        result = gaussian(&result, spec, limits)?;
    }
    if spec.passes > 0 || spec.fractional > 0 || spec.padding != [0, 0] {
        result = edge(&result, spec, limits)?;
    }
    Ok(result)
}

fn gaussian(mask: &Mask, spec: &BlurSpec, limits: &Limits) -> Result<Arc<Mask>, RenderError> {
    let (bounds, w, h) = extent(mask, spec.radius, limits)?;
    let n = w.checked_mul(h).ok_or_else(limit)?;
    let mut horizontal = zeros::<f32>(n)?;
    let mut output = zeros::<u8>(n)?;
    let kx = coefficients(spec.sigma[0], spec.radius[0])?;
    let ky = coefficients(spec.sigma[1], spec.radius[1])?;
    // No byte quantization between Gaussian axes. No FMA or fast math.
    for y in 0..mask.bounds().height() as usize {
        for x in 0..w {
            let mut sum = 0.0_f32;
            for (k, weight) in kx.iter().enumerate() {
                let sx = x as i64 + k as i64 - 2 * i64::from(spec.radius[0]);
                if sx >= 0 && sx < i64::from(mask.bounds().width()) {
                    sum += f32::from(mask.data()[y * mask.stride() + sx as usize]) * weight;
                }
            }
            horizontal[(y + spec.radius[1] as usize) * w + x] = sum;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0_f32;
            for (k, weight) in ky.iter().enumerate() {
                let sy = y as i64 + k as i64 - i64::from(spec.radius[1]);
                if sy >= 0 && sy < h as i64 {
                    sum += horizontal[sy as usize * w + x] * weight;
                }
            }
            // The reference scalar conversion uses +0.5. SIMD/MXCSR tail
            // equivalence is not asserted for this scalar A8 backend.
            output[y * w + x] = (sum + 0.5).floor().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(Arc::new(mask.with_data(bounds, w, output.into())?))
}

fn edge(mask: &Mask, spec: &BlurSpec, limits: &Limits) -> Result<Arc<Mask>, RenderError> {
    let (bounds, w, h) = extent(mask, spec.padding, limits)?;
    let n = w.checked_mul(h).ok_or_else(limit)?;
    let mut data = zeros::<u8>(n)?;
    let mut scratch = zeros::<u8>(n)?;
    for y in 0..mask.bounds().height() as usize {
        let start = (y + spec.padding[1] as usize) * w + spec.padding[0] as usize;
        data[start..start + mask.bounds().width() as usize].copy_from_slice(
            &mask.data()[y * mask.stride()..y * mask.stride() + mask.bounds().width() as usize],
        );
    }
    // Integer BE rounds only once after the 3x3 outer product, truncating /16.
    // The reference integer pass leaves the allocation's outermost ring intact.
    for _ in 0..spec.passes {
        scratch.copy_from_slice(&data);
        if w >= 3 && h >= 3 {
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let mut sum = 0_u32;
                    for (dy, wy) in [1_u32, 2, 1].into_iter().enumerate() {
                        for (dx, wx) in [1_u32, 2, 1].into_iter().enumerate() {
                            sum += u32::from(data[(y + dy - 1) * w + x + dx - 1]) * wy * wx;
                        }
                    }
                    scratch[y * w + x] = (sum >> 4) as u8;
                }
            }
        }
        std::mem::swap(&mut data, &mut scratch);
    }
    if spec.fractional != 0 {
        for vertical in [false, true] {
            for y in 0..h {
                for x in 0..w {
                    let center = u32::from(data[y * w + x]);
                    let (before, after) = if vertical {
                        (
                            if y == 0 {
                                0
                            } else {
                                u32::from(data[(y - 1) * w + x])
                            },
                            if y + 1 == h {
                                0
                            } else {
                                u32::from(data[(y + 1) * w + x])
                            },
                        )
                    } else {
                        (
                            if x == 0 {
                                0
                            } else {
                                u32::from(data[y * w + x - 1])
                            },
                            if x + 1 == w {
                                0
                            } else {
                                u32::from(data[y * w + x + 1])
                            },
                        )
                    };
                    let sum =
                        (before + after) * spec.fractional + center * (256 - 2 * spec.fractional);
                    // Actual scalar XyRounding<ROUND_HALF_TO_EVEN,8> expression,
                    // whose name must not be mistaken for IEEE ties-to-even.
                    scratch[y * w + x] = ((sum + 128 + ((sum >> 8) & 1)) >> 8).min(255) as u8;
                }
            }
            std::mem::swap(&mut data, &mut scratch);
        }
    }
    Ok(Arc::new(mask.with_data(bounds, w, data.into())?))
}
