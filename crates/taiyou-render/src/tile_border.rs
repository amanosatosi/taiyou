//! Exact native-grid ellipse dilation in bounded tiles. Only base intervals and
//! their compact support regions are retained; expanded spans are never built.
use crate::tile_fill::{ScanSpan, TileImage, TileStats, VerticalBand};
use std::ops::Range;
use taiyou_types::PixelRect;

const TILE: usize = 16;
const STACK: u64 = 16 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum Regions<'a> {
    Bands(&'a [VerticalBand]),
    Rows(&'a [ScanSpan]),
}
impl Regions<'_> {
    fn len(self) -> usize {
        match self {
            Self::Bands(b) => b.len(),
            Self::Rows(s) => s.len(),
        }
    }
    fn get(self, n: usize) -> Option<VerticalBand> {
        match self {
            Self::Bands(b) => b.get(n).copied(),
            Self::Rows(s) => s.get(n).map(|s| VerticalBand {
                first_y: s.y,
                last_y: s.y,
                left: s.left,
                right: s.right,
            }),
        }
    }
}
fn distance(y: i64, band: VerticalBand) -> usize {
    if y < band.first_y {
        (band.first_y - y) as usize
    } else {
        (y - band.last_y).max(0) as usize
    }
}
fn low_mask(bits: usize) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    }
}
fn set_range(bits: &mut [u64; 2], left: usize, right: usize) {
    if left >= right {
        return;
    }
    if left < 64 {
        bits[0] |= (u64::MAX << left) & low_mask(right);
    }
    if right > 64 {
        bits[1] |= (u64::MAX << left.saturating_sub(64)) & low_mask(right - 64);
    }
}

struct Raster<'a> {
    spans: &'a [ScanSpan],
    regions: Regions<'a>,
    arc: &'a [i64],
    source_y: i64,
    offsets: Vec<usize>,
    previous: Vec<bool>,
    indices: Vec<usize>,
    image: TileImage,
    allowance: u64,
    held: u64,
}
impl Raster<'_> {
    fn bytes(&self) -> Option<u64> {
        self.held
            .checked_add(STACK)?
            .checked_add(
                (self.offsets.capacity() as u64).checked_mul(std::mem::size_of::<usize>() as u64)?,
            )?
            .checked_add(self.previous.capacity() as u64)?
            .checked_add(
                (self.indices.capacity() as u64).checked_mul(std::mem::size_of::<usize>() as u64)?,
            )?
            .checked_add((self.image.tiles.capacity() as u64).checked_mul(256)?)
    }
    fn observe(&mut self) -> Option<()> {
        let bytes = self.bytes()?;
        if bytes > self.allowance {
            return None;
        }
        self.image.stats.scratch_peak = self.image.stats.scratch_peak.max(bytes);
        Some(())
    }
    fn append(&mut self, n: usize) -> Option<()> {
        if self.indices.len() == self.indices.capacity() {
            let additional = self.regions.len().max(1);
            let extra = (additional as u64).checked_mul(std::mem::size_of::<usize>() as u64)?;
            if self.bytes()?.checked_add(extra)? > self.allowance {
                return None;
            }
            self.indices.try_reserve_exact(additional).ok()?;
            self.observe()?;
        }
        self.indices.push(n);
        Some(())
    }
    fn native(&self, r: [usize; 4]) -> [i64; 4] {
        [
            i64::from(self.image.bounds.x0()) * 8 + r[0] as i64 * 8,
            i64::from(self.image.bounds.y0()) * 8 + r[1] as i64 * 8,
            i64::from(self.image.bounds.x0()) * 8 + r[2] as i64 * 8,
            i64::from(self.image.bounds.y0()) * 8 + r[3] as i64 * 8,
        ]
    }
    fn intersects(&self, band: VerticalBand, r: [i64; 4]) -> Option<bool> {
        let radius_y = self.arc.len() as i64 - 1;
        let first = r[1].max(band.first_y - radius_y);
        let last = (r[3] - 1).min(band.last_y + radius_y);
        if first > last {
            return Some(false);
        }
        let nearest = if last < band.first_y {
            distance(last, band)
        } else if first > band.last_y {
            distance(first, band)
        } else {
            0
        };
        let width = *self.arc.get(nearest)?;
        Some(band.left - width < r[2] && band.right + width > r[0])
    }
    fn covers(&self, band: VerticalBand, r: [i64; 4]) -> Option<bool> {
        let far = distance(r[1], band).max(distance(r[3] - 1, band));
        if far >= self.arc.len() {
            return Some(false);
        }
        let width = self.arc[far];
        Some(band.left - width <= r[0] && band.right + width >= r[2])
    }
    fn children(&mut self, ids: Range<usize>, r: [usize; 4]) -> Option<Range<usize>> {
        let native = self.native(r);
        let start = self.indices.len();
        for at in ids {
            let id = *self.indices.get(at)?;
            if self.intersects(self.regions.get(id)?, native)? {
                self.append(id)?;
            }
        }
        Some(start..self.indices.len())
    }
    fn uniform(&mut self, r: [usize; 4], fill: bool) -> Option<()> {
        for y in r[1] * 8..r[3] * 8 {
            let previous = self.previous.get_mut(y)?;
            if fill && !*previous {
                self.image.union_intervals = self.image.union_intervals.checked_add(1)?;
            }
            *previous = fill;
        }
        if fill {
            for y in r[1] / TILE..r[3].div_ceil(TILE) {
                for x in r[0] / TILE..r[2].div_ceil(TILE) {
                    self.image
                        .tiles
                        .get_mut(y * self.image.columns + x)?
                        .fill(64);
                }
            }
        }
        Some(())
    }
    fn node(&mut self, r: [usize; 4], ids: Range<usize>, depth: usize) -> Option<()> {
        if depth > 32 {
            return None;
        }
        if ids.is_empty() {
            self.image.stats.empty_regions += 1;
            return self.uniform(r, false);
        }
        let native = self.native(r);
        for at in ids.clone() {
            if self.covers(self.regions.get(*self.indices.get(at)?)?, native)? {
                self.image.stats.solid_regions += 1;
                return self.uniform(r, true);
            }
        }
        let width = r[2] - r[0];
        let height = r[3] - r[1];
        if width <= TILE && height <= TILE {
            return self.leaf(r, ids);
        }
        let mut a = r;
        let mut b = r;
        if width >= height && width > TILE {
            let middle = r[0] + (width.div_ceil(TILE) / 2) * TILE;
            a[2] = middle;
            b[0] = middle;
        } else {
            let middle = r[1] + (height.div_ceil(TILE) / 2) * TILE;
            a[3] = middle;
            b[1] = middle;
        }
        let mark = self.indices.len();
        let left = self.children(ids.clone(), a)?;
        let right = self.children(ids, b)?;
        self.node(a, left, depth + 1)?;
        self.node(b, right, depth + 1)?;
        self.indices.truncate(mark);
        Some(())
    }
    fn source_row(&self, y: i64) -> &[ScanSpan] {
        let Ok(row) = usize::try_from(y - self.source_y) else {
            return &[];
        };
        let (Some(&start), Some(&end)) = (self.offsets.get(row), self.offsets.get(row + 1)) else {
            return &[];
        };
        &self.spans[start..end]
    }
    fn add_source_row(&self, bits: &mut [u64; 2], y: i64, width: i64, x0: i64, x1: i64) {
        let row = self.source_row(y);
        let first = row.partition_point(|s| s.right + width <= x0);
        for span in &row[first..] {
            if span.left - width >= x1 {
                break;
            }
            let left = (span.left - width).max(x0) - x0;
            let right = (span.right + width).min(x1) - x0;
            if left < right {
                set_range(bits, left as usize, right as usize);
            }
        }
    }
    fn leaf(&mut self, r: [usize; 4], ids: Range<usize>) -> Option<()> {
        let native = self.native(r);
        let count = (r[2] - r[0]) * 8;
        let full = [low_mask(count), low_mask(count.saturating_sub(64))];
        let single = if ids.len() == 1 {
            Some(self.regions.get(*self.indices.get(ids.start)?)?)
        } else {
            None
        };
        if single.is_some() {
            self.image.stats.simple_border_tiles += 1;
        } else {
            self.image.stats.complex_tiles += 1;
        }
        for y in native[1]..native[3] {
            let mut bits = [0_u64; 2];
            if let Some(band) = single {
                if let Some(&width) = self.arc.get(distance(y, band)) {
                    let left = (band.left - width).max(native[0]) - native[0];
                    let right = (band.right + width).min(native[2]) - native[0];
                    if left < right {
                        set_range(&mut bits, left as usize, right as usize);
                    }
                }
            } else {
                // Positive union is saturated once the complete native row is
                // set. Further contributors provably cannot change any sample.
                for distance in 0..self.arc.len() {
                    let width = self.arc[distance];
                    self.add_source_row(
                        &mut bits,
                        y - distance as i64,
                        width,
                        native[0],
                        native[2],
                    );
                    if distance != 0 {
                        self.add_source_row(
                            &mut bits,
                            y + distance as i64,
                            width,
                            native[0],
                            native[2],
                        );
                    }
                    if bits == full {
                        break;
                    }
                }
            }
            let row = usize::try_from(y - i64::from(self.image.bounds.y0()) * 8).ok()?;
            let mut previous = *self.previous.get(row)?;
            for (word, valid) in [
                (bits[0], count.min(64)),
                (bits[1], count.saturating_sub(64)),
            ] {
                if valid == 0 {
                    continue;
                }
                let starts = word & !((word << 1) | u64::from(previous));
                self.image.union_intervals = self
                    .image
                    .union_intervals
                    .checked_add(u64::from(starts.count_ones()))?;
                previous = ((word >> (valid - 1)) & 1) != 0;
            }
            self.previous[row] = previous;
            let tile_index = (r[1] / TILE) * self.image.columns + r[0] / TILE;
            let tile = self.image.tiles.get_mut(tile_index)?;
            let pixel_y = row / 8 - r[1];
            for x in 0..r[2] - r[0] {
                let coverage = ((bits[x / 8] >> ((x % 8) * 8)) & 255).count_ones() as u8;
                let pixel = tile.get_mut(pixel_y * TILE + x)?;
                *pixel = pixel.checked_add(coverage).filter(|&v| v <= 64)?;
            }
        }
        Some(())
    }
}

pub(crate) fn rasterize(
    spans: &[ScanSpan],
    regions: Regions<'_>,
    arc: &[i64],
    bounds: PixelRect,
    allowance: u64,
    held: u64,
) -> Option<TileImage> {
    if spans.is_empty()
        || arc.is_empty()
        || arc.iter().any(|&w| w < 0)
        || !arc.windows(2).all(|w| w[0] >= w[1])
    {
        return None;
    }
    if !spans
        .windows(2)
        .all(|w| w[0].y < w[1].y || (w[0].y == w[1].y && w[0].right <= w[1].left))
    {
        return None;
    }
    let width = bounds.width() as usize;
    let height = bounds.height() as usize;
    if width == 0 || height == 0 {
        return None;
    }
    let native = [
        i64::from(bounds.x0()) * 8,
        i64::from(bounds.y0()) * 8,
        i64::from(bounds.x1()) * 8,
        i64::from(bounds.y1()) * 8,
    ];
    let radius_y = arc.len() as i64 - 1;
    for n in 0..regions.len() {
        let band = regions.get(n)?;
        if band.first_y - radius_y < native[1]
            || band.last_y + radius_y >= native[3]
            || band.left - arc[0] < native[0]
            || band.right + arc[0] > native[2]
        {
            return None;
        }
    }
    let source_y = spans[0].y;
    let source_rows = usize::try_from(spans.last()?.y - source_y + 1).ok()?;
    let columns = width.div_ceil(TILE);
    let tiles = columns.checked_mul(height.div_ceil(TILE))?;
    let rows = height.checked_mul(8)?;
    let planned = held
        .checked_add(STACK)?
        .checked_add((source_rows as u64 + 1).checked_mul(std::mem::size_of::<usize>() as u64)?)?
        .checked_add(rows as u64)?
        .checked_add((tiles as u64).checked_mul(256)?)?
        .checked_add((regions.len() as u64).checked_mul(std::mem::size_of::<usize>() as u64)?)?;
    if planned > allowance {
        return None;
    }
    let mut offsets = Vec::new();
    offsets.try_reserve_exact(source_rows + 1).ok()?;
    let mut previous = Vec::new();
    previous.try_reserve_exact(rows).ok()?;
    let mut indices = Vec::new();
    indices.try_reserve_exact(regions.len()).ok()?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(tiles).ok()?;
    let mut raster = Raster {
        spans,
        regions,
        arc,
        source_y,
        offsets,
        previous,
        indices,
        image: TileImage {
            bounds,
            columns,
            tiles: pixels,
            source_intervals: spans.len() as u64,
            union_intervals: 0,
            stats: TileStats::default(),
        },
        allowance,
        held,
    };
    raster.observe()?;
    let mut cursor = 0;
    for row in 0..source_rows {
        raster.offsets.push(cursor);
        while cursor < spans.len() && spans[cursor].y == source_y + row as i64 {
            cursor += 1;
        }
    }
    raster.offsets.push(cursor);
    raster.previous.resize(rows, false);
    raster.image.tiles.resize(tiles, [0; 256]);
    raster.indices.extend(0..regions.len());
    raster.node([0, 0, width, height], 0..regions.len(), 0)?;
    Some(raster.image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taiyou_types::*;

    #[test]
    fn tiled_ellipse_union_keeps_pixels_work_and_rejections() {
        let mut successes = 0;
        let mut rejections = 0;
        for phase in [0, 7, 31, 63] {
            for shape in 0..3 {
                let p = |x: i32, y: i32| {
                    DevicePoint::new(
                        Q6::from_ticks(x - 7313 + phase),
                        Q6::from_ticks(y - 2137 + phase),
                    )
                };
                let mut ops = vec![PathOp::MoveTo(p(0, 0))];
                match shape {
                    0 => ops.extend([
                        PathOp::LineTo(p(10240, 0)),
                        PathOp::LineTo(p(10240, 8192)),
                        PathOp::LineTo(p(0, 8192)),
                    ]),
                    1 => ops.extend([
                        PathOp::CubicTo(p(3000, -2000), p(11000, 9000), p(10240, 4000)),
                        PathOp::QuadTo(p(12000, 12000), p(0, 8192)),
                        PathOp::LineTo(p(4096, 4096)),
                    ]),
                    _ => ops.extend([
                        PathOp::LineTo(p(10240, 8192)),
                        PathOp::LineTo(p(0, 8000)),
                        PathOp::LineTo(p(10113, 17)),
                    ]),
                }
                ops.push(PathOp::Close);
                if shape == 0 {
                    ops.extend([
                        PathOp::MoveTo(p(2000, 2000)),
                        PathOp::LineTo(p(2000, 6000)),
                        PathOp::LineTo(p(8000, 6000)),
                        PathOp::LineTo(p(8000, 2000)),
                        PathOp::Close,
                    ]);
                }
                for (rx, ry) in [(64, 64), (768, 512), (512, 768), (0, 512), (512, 0), (1, 1)] {
                    for (memory, global_work, remaining) in [
                        (512 * 1024 * 1024, 32_000_000, 32_000_000),
                        (64 * 1024 * 1024, 1_000_000, 1_000_000),
                        (4096, 1_000_000, 1_000_000),
                        (65536, 1_000_000, 1_000_000),
                        (1048576, 1_000_000, 1_000_000),
                        (512 * 1024 * 1024, 512, 512),
                        (512 * 1024 * 1024, 1_000_000, 512),
                        (512 * 1024 * 1024, 4096, 1_000_000),
                    ] {
                        let limits = Limits::default()
                            .with_transient_bytes(memory)
                            .unwrap()
                            .with_path_segments_per_frame(global_work)
                            .unwrap();
                        let mut old = crate::timing::StageTimings::default();
                        let mut new = crate::timing::StageTimings::default();
                        let a = crate::axis_widen::rasterize_legacy_axis_timed(
                            &ops,
                            Q6::from_ticks(rx),
                            Q6::from_ticks(ry),
                            remaining,
                            &limits,
                            &mut old,
                        );
                        let b = crate::axis_widen::rasterize_axis_xy_timed(
                            &ops,
                            Q6::from_ticks(rx),
                            Q6::from_ticks(ry),
                            remaining,
                            &limits,
                            &mut new,
                        );
                        match (a, b) {
                            (Ok((a, aw)), Ok((b, bw))) => {
                                assert_eq!(
                                    (a.bounds(), a.stride(), a.is_xy_coverage(), aw),
                                    (b.bounds(), b.stride(), b.is_xy_coverage(), bw)
                                );
                                assert_eq!(
                                    a.data(),
                                    b.data(),
                                    "phase={phase} shape={shape} radii={rx},{ry}"
                                );
                                assert_eq!(
                                    (old.raster_peak_bytes, old.raster_required_work),
                                    (new.raster_peak_bytes, new.raster_required_work)
                                );
                                if memory == 512 * 1024 * 1024 && global_work == 32_000_000 {
                                    assert_eq!(
                                        new.tiled_borders, 1,
                                        "proof unexpectedly fell back"
                                    );
                                    successes += 1;
                                }
                            }
                            (Err(a), Err(b)) => {
                                assert_eq!(
                                    (a.code(), a.resource_failure(), a.detail()),
                                    (b.code(), b.resource_failure(), b.detail())
                                );
                                rejections += 1;
                            }
                            _ => panic!(
                                "border result differs phase={phase} shape={shape} radii={rx},{ry} memory={memory} global={global_work} remaining={remaining}"
                            ),
                        }
                    }
                }
            }
        }
        assert_eq!(successes, 72);
        assert!(rejections > 0);
        println!(
            "exact tiled borders: {successes} fully admitted cases, {rejections} matching rejections"
        );
    }

    #[test]
    fn bit_ranges_keep_every_native_boundary_sample() {
        for left in 0..128 {
            for right in left + 1..=128 {
                let mut bits = [0; 2];
                set_range(&mut bits, left, right);
                for x in 0..128 {
                    assert_eq!(
                        (bits[x / 64] >> (x % 64)) & 1,
                        u64::from((left..right).contains(&x))
                    );
                }
            }
        }
    }
}
