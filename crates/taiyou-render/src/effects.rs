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
#[derive(Clone, Debug, PartialEq)]
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

/// Preserve the original full combined extent/scratch checks on retained hits.
pub(crate) fn preflight_effects(
    mask: &Mask,
    spec: &BlurSpec,
    limits: &Limits,
) -> Result<(), RenderError> {
    if spec.active && !mask.bounds().is_empty() {
        extent(mask, spec.support(), limits)?;
    }
    Ok(())
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
    preflight_effects(mask, spec, limits)?;
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
    let native = mask.is_xy_coverage();
    // Interchange independent pixel/tap loops without changing ANY pixel's
    // f32 addition order. No byte quantization, FMA or fast math between axes.
    let source_width = mask.bounds().width() as usize;
    for (y, source) in mask.data().chunks(mask.stride())
        .take(mask.bounds().height() as usize).enumerate()
    {
        let start = (y + spec.radius[1] as usize) * w;
        let target = &mut horizontal[start..start + w];
        for i in 0..kx.len() {
            let k = gaussian_tap(i, kx.len(), native);
            let weight = kx[k];
            // Full-support padding proves this entire contiguous source row
            // fits: shift is 2*radius-k, with k in 0..=2*radius.
            let shift = 2 * spec.radius[0] as usize - k;
            for (sum, &value) in target[shift..shift + source_width]
                .iter_mut().zip(&source[..source_width])
            {
                *sum += f32::from(value) * weight;
            }
        }
    }
    // One reusable row fits the existing preflight's two-float-plane allowance.
    let mut sums = zeros::<f32>(w)?;
    for (y, target) in output.chunks_exact_mut(w).enumerate() {
        sums.fill(0.0);
        for i in 0..ky.len() {
            let k = gaussian_tap(i, ky.len(), native);
            let weight = ky[k];
            let sy = y as i64 + k as i64 - i64::from(spec.radius[1]);
            if sy >= 0 && sy < h as i64 {
                let start = sy as usize * w;
                for (sum, &value) in sums.iter_mut().zip(&horizontal[start..start + w]) {
                    *sum += value * weight;
                }
            }
        }
        for (x, (pixel, &sum)) in target.iter_mut().zip(&sums).enumerate() {
            // Keep native complete-16-column nearest/even and tail +0.5.
            *pixel = gaussian_byte(sum, native && x < w / 16 * 16);
        }
    }
    Ok(Arc::new(mask.with_data(bounds, w, output.into())?))
}

fn gaussian_tap(i: usize, length: usize, native: bool) -> usize {
    // xy_filter.cpp's general float filter accumulates each four-tap group
    // in 0,2,1,3 order. Its symmetric three-tap specialization uses 0,1,2.
    if native && length > 3 {
        (i & !3) + [0, 2, 1, 3][i & 3]
    } else {
        i
    }
}

fn gaussian_byte(sum: f32, nearest_even: bool) -> u8 {
    let value = if nearest_even {
        sum.round_ties_even()
    } else {
        (sum + 0.5).floor()
    };
    value.clamp(0.0, 255.0) as u8
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

#[cfg(test)]
mod gaussian_conversion_tests {
    use super::gaussian_byte;

    #[test]
    fn native_blocks_keep_the_xy_nearest_even_tie_and_scalar_tail() {
        assert_eq!(gaussian_byte(12.5, true), 12);
        assert_eq!(gaussian_byte(12.5, false), 13);
        assert_eq!(gaussian_byte(13.5, true), 14);
        assert_eq!(gaussian_byte(13.5, false), 14);
        for native in [true, false] {
            assert_eq!(gaussian_byte(0.0, native), 0);
            assert_eq!(gaussian_byte(64.0, native), 64);
            assert_eq!(gaussian_byte(255.0, native), 255);
        }
    }
}

#[cfg(test)]
mod gaussian_row_tests {
    use super::*;

    // Independent retained per-pixel producer; never derive expected bytes
    // through the candidate's row accumulation.
    fn scalar(mask: &Mask, spec: &BlurSpec, limits: &Limits) -> Arc<Mask> {
        let (bounds, w, h) = extent(mask, spec.radius, limits).unwrap();
        let mut horizontal = vec![0_f32; w * h];
        let mut output = vec![0_u8; w * h];
        let kx = coefficients(spec.sigma[0], spec.radius[0]).unwrap();
        let ky = coefficients(spec.sigma[1], spec.radius[1]).unwrap();
        let native = mask.is_xy_coverage();
        for y in 0..mask.bounds().height() as usize {
            for x in 0..w {
                let mut sum = 0_f32;
                for i in 0..kx.len() {
                    let k = gaussian_tap(i, kx.len(), native);
                    let sx = x as i64 + k as i64 - 2 * i64::from(spec.radius[0]);
                    if sx >= 0 && sx < i64::from(mask.bounds().width()) {
                        sum += f32::from(mask.data()[y * mask.stride() + sx as usize]) * kx[k];
                    }
                }
                horizontal[(y + spec.radius[1] as usize) * w + x] = sum;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0_f32;
                for i in 0..ky.len() {
                    let k = gaussian_tap(i, ky.len(), native);
                    let sy = y as i64 + k as i64 - i64::from(spec.radius[1]);
                    if sy >= 0 && sy < h as i64 {
                        sum += horizontal[sy as usize * w + x] * ky[k];
                    }
                }
                output[y * w + x] = gaussian_byte(sum, native && x < w / 16 * 16);
            }
        }
        Arc::new(mask.with_data(bounds, w, output.into()).unwrap())
    }

    #[test]
    fn contiguous_rows_preserve_order_units_stride_halos_and_native_tail() {
        let limits = Limits::default();
        let mut cases = 0;
        for native in [false, true] {
            for shape in 0..12 {
                let w = [1, 7, 15, 16, 17, 31, 32, 63][shape % 8];
                let h = [1, 3, 9, 17][shape % 4];
                let stride = w + shape % 5;
                let maximum = if native {64} else {255};
                let mut seed = 71_u32;
                let data: Vec<u8> = (0..stride * h).map(|i| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    match shape % 3 {
                        0 => if i % 13 == 0 {maximum} else {0},
                        1 => if i % 2 == 0 {maximum} else {0},
                        _ => (seed % (u32::from(maximum) + 1)) as u8,
                    }
                }).collect();
                let bounds = PixelRect::new(-37, -19, -37 + w as i32, -19 + h as i32).unwrap();
                let mask = if native { Mask::new_xy(bounds, stride, data.into()) }
                    else { Mask::new(bounds, stride, data.into()) }.unwrap();
                for blur in [0.1, 0.34, 0.75, 1.0, 2.3, 6.0, 12.0] {
                    let spec = BlurSpec::new(FiniteF64::new(blur).unwrap(),
                        FiniteF64::new(1.5).unwrap(), [0.75, 1.25], &limits).unwrap();
                    let a = scalar(&mask, &spec, &limits);
                    let b = gaussian(&mask, &spec, &limits).unwrap();
                    assert_eq!((a.bounds(), a.stride(), a.is_xy_coverage()),
                        (b.bounds(), b.stride(), b.is_xy_coverage()));
                    assert_eq!(a.data(), b.data(), "native={native} shape={shape} blur={blur}");
                    cases += 1;
                }
            }
        }
        assert_eq!(cases, 168);
    }
}
