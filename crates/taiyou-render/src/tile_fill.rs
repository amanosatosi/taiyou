//! Serial tiled native-eighth fill. Geometry/flattening and resource policy are
//! supplied by axis_widen; no analytic libass coverage is used here.
use std::ops::Range;
use taiyou_types::PixelRect;

const TILE: usize = 16;
const TILE_BYTES: usize = TILE * TILE;
const STACK_ALLOWANCE: u64 = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScanSpan {
    pub y: i64,
    pub left: i64,
    pub right: i64,
}
#[derive(Clone, Copy)]
pub(crate) struct VerticalBand {
    pub first_y: i64,
    pub last_y: i64,
    pub left: i64,
    pub right: i64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Segment {
    a: [i64; 2],
    slope: i128,
    first: i64,
    last: i64,
    winding: i32,
}
impl Segment {
    pub(crate) fn new(mut a: [i64; 2], mut b: [i64; 2]) -> Option<Self> {
        if a[1] == b[1] {
            return None;
        }
        let winding = if a[1] < b[1] { 1 } else { -1 };
        if a[1] > b[1] {
            std::mem::swap(&mut a, &mut b);
        }
        let first = (a[1] + 3).div_euclid(8);
        let last = (b[1] - 5).div_euclid(8);
        if first > last {
            return None;
        }
        Some(Self {
            a,
            slope: (i128::from(b[0] - a[0]) * 65536) / i128::from(b[1] - a[1]),
            first,
            last,
            winding,
        })
    }
    pub(crate) fn rows(self) -> u64 {
        (self.last - self.first + 1) as u64
    }
    fn crossing(self, y: i64) -> Option<i64> {
        let sample = y * 8 + 4;
        let x16 = i128::from(self.a[0]) * 8192
            + (self.slope * i128::from(sample - self.a[1])).div_euclid(8);
        i64::try_from((x16 + 32768).div_euclid(65536)).ok()
    }
    fn intersects(self, r: [i64; 4]) -> Option<bool> {
        let first = self.first.max(r[1]);
        let last = self.last.min(r[3] - 1);
        if first > last {
            return Some(false);
        }
        let a = self.crossing(first)?;
        let b = self.crossing(last)?;
        Some(a.max(b) >= r[0] && a.min(b) < r[2])
    }
}

#[derive(Clone, Copy, Default)]
struct Row {
    winding: i64,
    start: i64,
    last_end: i64,
    has_interval: bool,
}
#[derive(Clone, Copy)]
struct Crossing {
    x: i64,
    winding: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TileStats {
    pub empty_regions: u64,
    pub solid_regions: u64,
    pub halfplane_tiles: u64,
    pub complex_tiles: u64,
    pub simple_border_tiles: u64,
    pub scratch_peak: u64,
}
pub(crate) struct TileImage {
    pub bounds: PixelRect,
    pub columns: usize,
    pub tiles: Vec<[u8; TILE_BYTES]>,
    pub source_intervals: u64,
    pub union_intervals: u64,
    pub stats: TileStats,
}
impl TileImage {
    pub(crate) fn into_pixels(self, allowance: u64) -> Option<Vec<u8>> {
        let width = self.bounds.width() as usize;
        let height = self.bounds.height() as usize;
        let length = width.checked_mul(height)?;
        let live = (self.tiles.capacity() as u64).checked_mul(TILE_BYTES as u64)?;
        // Callers publish Vec storage through Arc; include its temporary copy.
        if live.checked_add((length as u64).checked_mul(2)?)? > allowance {
            return None;
        }
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(length).ok()?;
        if live
            .checked_add(pixels.capacity() as u64)?
            .checked_add(length as u64)?
            > allowance
        {
            return None;
        }
        pixels.resize(length, 0);
        for y in 0..height {
            for x in (0..width).step_by(TILE) {
                let count = TILE.min(width - x);
                let tile = self.tiles.get((y / TILE) * self.columns + x / TILE)?;
                let start = (y % TILE) * TILE;
                pixels[y * width + x..y * width + x + count]
                    .copy_from_slice(&tile[start..start + count]);
            }
        }
        Some(pixels)
    }
}

struct Raster<'a> {
    segments: &'a [Segment],
    image: TileImage,
    rows: Vec<Row>,
    indices: Vec<usize>,
    crossings: Vec<Crossing>,
    allowance: u64,
    held: u64,
}
impl Raster<'_> {
    fn bytes(&self) -> Option<u64> {
        self.held
            .checked_add(STACK_ALLOWANCE)?
            .checked_add((self.image.tiles.capacity() as u64).checked_mul(TILE_BYTES as u64)?)?
            .checked_add(
                (self.rows.capacity() as u64).checked_mul(std::mem::size_of::<Row>() as u64)?,
            )?
            .checked_add(
                (self.indices.capacity() as u64).checked_mul(std::mem::size_of::<usize>() as u64)?,
            )?
            .checked_add(
                (self.crossings.capacity() as u64)
                    .checked_mul(std::mem::size_of::<Crossing>() as u64)?,
            )
    }
    fn observe(&mut self) -> Option<()> {
        let bytes = self.bytes()?;
        if bytes > self.allowance {
            return None;
        }
        self.image.stats.scratch_peak = self.image.stats.scratch_peak.max(bytes);
        Some(())
    }
    fn append(&mut self, index: usize) -> Option<()> {
        if self.indices.len() == self.indices.capacity() {
            // Explicit exact growth keeps optional arena allocations bounded.
            let additional = self.segments.len().max(1);
            let extra = (additional as u64).checked_mul(std::mem::size_of::<usize>() as u64)?;
            if self.bytes()?.checked_add(extra)? > self.allowance {
                return None;
            }
            self.indices.try_reserve_exact(additional).ok()?;
            self.observe()?;
        }
        self.indices.push(index);
        Some(())
    }
    fn native_rect(&self, r: [usize; 4]) -> [i64; 4] {
        [
            i64::from(self.image.bounds.x0()) * 8 + r[0] as i64 * 8,
            i64::from(self.image.bounds.y0()) * 8 + r[1] as i64 * 8,
            i64::from(self.image.bounds.x0()) * 8 + r[2] as i64 * 8,
            i64::from(self.image.bounds.y0()) * 8 + r[3] as i64 * 8,
        ]
    }
    fn children(&mut self, ids: Range<usize>, r: [usize; 4]) -> Option<Range<usize>> {
        let native = self.native_rect(r);
        let start = self.indices.len();
        for at in ids {
            let index = *self.indices.get(at)?;
            if self.segments.get(index)?.intersects(native)? {
                self.append(index)?;
            }
        }
        Some(start..self.indices.len())
    }
    fn fill_solid(&mut self, r: [usize; 4]) -> Option<()> {
        for y in (r[1] / TILE)..r[3].div_ceil(TILE) {
            for x in (r[0] / TILE)..r[2].div_ceil(TILE) {
                self.image
                    .tiles
                    .get_mut(y * self.image.columns + x)?
                    .fill(64);
            }
        }
        Some(())
    }
    fn node(&mut self, r: [usize; 4], ids: Range<usize>, depth: usize) -> Option<()> {
        if depth > 32 {
            return None;
        }
        if ids.is_empty() {
            let rows = self.rows.get(r[1] * 8..r[3] * 8)?;
            if rows.iter().all(|row| row.winding == 0) {
                self.image.stats.empty_regions += 1;
                return Some(());
            }
            if rows.iter().all(|row| row.winding != 0) {
                self.image.stats.solid_regions += 1;
                return self.fill_solid(r);
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
            let tiles = width.div_ceil(TILE);
            let middle = r[0] + (tiles / 2) * TILE;
            a[2] = middle;
            b[0] = middle;
        } else {
            let tiles = height.div_ceil(TILE);
            let middle = r[1] + (tiles / 2) * TILE;
            a[3] = middle;
            b[1] = middle;
        }
        let mark = self.indices.len();
        let left = self.children(ids.clone(), a)?;
        let right = self.children(ids, b)?;
        // Left/top first: every native row sees crossings in global x order.
        self.node(a, left, depth + 1)?;
        self.node(b, right, depth + 1)?;
        self.indices.truncate(mark);
        Some(())
    }
    fn write(&mut self, r: [usize; 4], native_y: i64, left: i64, right: i64) -> Option<()> {
        if left >= right {
            return Some(());
        }
        let origin_x = i64::from(self.image.bounds.x0()) * 8;
        let origin_y = i64::from(self.image.bounds.y0()) * 8;
        let y = usize::try_from((native_y - origin_y).div_euclid(8)).ok()?;
        let tile_index = (r[1] / TILE) * self.image.columns + r[0] / TILE;
        let tile = self.image.tiles.get_mut(tile_index)?;
        for x in left.div_euclid(8)..-(-right).div_euclid(8) {
            let amount = (right.min((x + 1) * 8) - left.max(x * 8)) as u8;
            let local_x = usize::try_from(x - origin_x.div_euclid(8)).ok()? - r[0];
            let destination = tile.get_mut((y - r[1]) * TILE + local_x)?;
            *destination = destination.checked_add(amount).filter(|&n| n <= 64)?;
        }
        Some(())
    }
    fn leaf(&mut self, r: [usize; 4], ids: Range<usize>) -> Option<()> {
        if ids.len() == 1 {
            self.image.stats.halfplane_tiles += 1;
        } else if !ids.is_empty() {
            self.image.stats.complex_tiles += 1;
        }
        let native = self.native_rect(r);
        for y in native[1]..native[3] {
            self.crossings.clear();
            for at in ids.clone() {
                let segment = *self.segments.get(*self.indices.get(at)?)?;
                if y < segment.first || y > segment.last {
                    continue;
                }
                let x = segment.crossing(y)?;
                if x >= native[0] && x < native[2] {
                    self.crossings.push(Crossing {
                        x,
                        winding: segment.winding,
                    });
                }
            }
            if self.crossings.len() > 1 {
                self.crossings.sort_unstable_by_key(|e| (e.x, e.winding));
            }
            let row_index = usize::try_from(y - i64::from(self.image.bounds.y0()) * 8).ok()?;
            let mut row = *self.rows.get(row_index)?;
            let mut left = native[0];
            for n in 0..self.crossings.len() {
                let crossing = self.crossings[n];
                if row.winding != 0 {
                    self.write(r, y, left, crossing.x)?;
                }
                if row.winding == 0 {
                    row.start = crossing.x;
                }
                row.winding += i64::from(crossing.winding);
                if row.winding == 0 && row.start < crossing.x {
                    self.image.source_intervals = self.image.source_intervals.checked_add(1)?;
                    if !row.has_interval || row.last_end < row.start {
                        self.image.union_intervals = self.image.union_intervals.checked_add(1)?;
                    }
                    row.last_end = if row.has_interval {
                        row.last_end.max(crossing.x)
                    } else {
                        crossing.x
                    };
                    row.has_interval = true;
                }
                left = crossing.x;
            }
            if row.winding != 0 {
                self.write(r, y, left, native[2])?;
            }
            self.rows[row_index] = row;
        }
        Some(())
    }
}

/// None means optional tiled scratch/preconditions could not be proved; callers
/// retain the legacy producer. Nothing points into the request-local arenas.
pub(crate) fn rasterize(
    segments: &[Segment],
    bounds: PixelRect,
    allowance: u64,
    held: u64,
) -> Option<TileImage> {
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
    // Required for exact legacy interval/resource counts, including contours
    // outside the final viewport: no crossing may be cropped by this producer.
    for segment in segments {
        let a = segment.crossing(segment.first)?;
        let b = segment.crossing(segment.last)?;
        if segment.first < native[1]
            || segment.last >= native[3]
            || a.min(b) < native[0]
            || a.max(b) >= native[2]
        {
            return None;
        }
    }
    let columns = width.div_ceil(TILE);
    let count = columns.checked_mul(height.div_ceil(TILE))?;
    let row_count = height.checked_mul(8)?;
    let planned = (count as u64)
        .checked_mul(TILE_BYTES as u64)?
        .checked_add((row_count as u64).checked_mul(std::mem::size_of::<Row>() as u64)?)?
        .checked_add((segments.len() as u64).checked_mul(
            (std::mem::size_of::<usize>() + std::mem::size_of::<Crossing>()) as u64,
        )?)?
        .checked_add(held)?
        .checked_add(STACK_ALLOWANCE)?;
    if planned > allowance {
        return None;
    }
    let mut tiles = Vec::new();
    tiles.try_reserve_exact(count).ok()?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(row_count).ok()?;
    let mut indices = Vec::new();
    indices.try_reserve_exact(segments.len()).ok()?;
    let mut crossings = Vec::new();
    crossings.try_reserve_exact(segments.len()).ok()?;
    let mut raster = Raster {
        segments,
        image: TileImage {
            bounds,
            columns,
            tiles,
            source_intervals: 0,
            union_intervals: 0,
            stats: Default::default(),
        },
        rows,
        indices,
        crossings,
        allowance,
        held,
    };
    raster.observe()?;
    raster.image.tiles.resize(count, [0; TILE_BYTES]);
    raster.rows.resize(row_count, Row::default());
    raster.indices.extend(0..segments.len());
    raster.node([0, 0, width, height], 0..segments.len(), 0)?;
    if raster.rows.iter().any(|row| row.winding != 0) {
        return None;
    }
    Some(raster.image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taiyou_types::*;

    fn point(p: [i64; 2]) -> DevicePoint {
        DevicePoint::new(Q6::from_ticks(p[0] as i32), Q6::from_ticks(p[1] as i32))
    }
    fn check(contours: &[Vec<[i64; 2]>]) -> TileStats {
        let mut ops = Vec::new();
        let mut segments = Vec::new();
        for contour in contours {
            let first = contour[0];
            ops.push(PathOp::MoveTo(point(first)));
            for pair in contour.windows(2) {
                ops.push(PathOp::LineTo(point(pair[1])));
                if let Some(segment) = Segment::new(pair[0], pair[1]) {
                    segments.push(segment);
                }
            }
            ops.push(PathOp::Close);
            if let Some(segment) = Segment::new(*contour.last().unwrap(), first) {
                segments.push(segment);
            }
        }
        let limits = Limits::default()
            .with_transient_bytes(512 * 1024 * 1024)
            .unwrap()
            .with_path_segments_per_frame(32_000_000)
            .unwrap();
        let mut observed = crate::timing::StageTimings::default();
        let expected =
            crate::axis_widen::rasterize_legacy_body_timed(&ops, &limits, &mut observed).unwrap();
        assert!(segments.iter().map(|s| s.rows()).sum::<u64>() > 0);
        let image = rasterize(&segments, expected.bounds(), limits.transient_bytes(), 0).unwrap();
        let stats = image.stats;
        // Independent full-row reference for the legacy source/union work counts.
        let mut edges = Vec::new();
        for segment in &segments {
            for y in segment.first..=segment.last {
                edges.push((y, segment.crossing(y).unwrap(), segment.winding));
            }
        }
        edges.sort_unstable();
        let mut row = None;
        let mut winding = 0;
        let mut start = 0;
        let mut source = 0;
        let mut union = 0;
        let mut last_end = None;
        for (y, x, w) in edges {
            if row != Some(y) {
                row = Some(y);
                winding = 0;
                last_end = None;
            }
            if winding == 0 {
                start = x;
            }
            winding += w;
            if winding == 0 && start < x {
                source += 1;
                if last_end.is_none_or(|end| end < start) {
                    union += 1;
                }
                last_end = Some(last_end.map_or(x, |end: i64| end.max(x)));
            }
        }
        assert_eq!(image.source_intervals, source);
        assert_eq!(image.union_intervals, union);
        let edge_count = segments.iter().map(|s| s.rows()).sum();
        let count = contours.iter().map(|c| c.len() as u64 - 1).sum();
        let witness = crate::axis_widen::legacy_fill_witness(
            ops.len(),
            count,
            edge_count,
            source,
            union,
            expected.bounds(),
            &limits,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            (witness.peak, witness.required_work),
            (observed.raster_peak_bytes, observed.raster_required_work)
        );
        for (memory, work) in [
            (4096, 1_000_000),
            (8192, 1_000_000),
            (65536, 1_000_000),
            (262144, 1_000_000),
            (524288, 1_000_000),
            (512 * 1024 * 1024, 64),
            (512 * 1024 * 1024, 1024),
            (512 * 1024 * 1024, 4096),
        ] {
            let restricted = limits
                .clone()
                .with_transient_bytes(memory)
                .unwrap()
                .with_path_segments_per_frame(work)
                .unwrap();
            let mut actual_witness = crate::timing::StageTimings::default();
            let actual = crate::axis_widen::rasterize_legacy_body_timed(
                &ops,
                &restricted,
                &mut actual_witness,
            );
            let planned = crate::axis_widen::legacy_fill_witness(
                ops.len(),
                count,
                edge_count,
                source,
                union,
                expected.bounds(),
                &restricted,
            );
            match (actual, planned) {
                (Ok(_), Ok(Some(planned))) => assert_eq!(
                    (planned.peak, planned.required_work),
                    (
                        actual_witness.raster_peak_bytes,
                        actual_witness.raster_required_work
                    )
                ),
                (Err(actual), Err(planned)) => assert_eq!(
                    (actual.code(), actual.resource_failure(), actual.detail()),
                    (planned.code(), planned.resource_failure(), planned.detail())
                ),
                _ => panic!("legacy resource proof differs at memory={memory}, work={work}"),
            }
        }
        let actual = image.into_pixels(limits.transient_bytes()).unwrap();
        assert_eq!(actual, expected.data(), "native eighth coverage changed");
        assert!(stats.scratch_peak <= limits.transient_bytes());
        stats
    }

    #[test]
    fn recursive_tiles_match_native_fill_for_phase_holes_overlap_and_self_intersection() {
        let mut kinds = TileStats::default();
        for phase in [0, 1, 7, 31, 63] {
            for offset in [[0, 0], [-177 * 64, -93 * 64], [97 * 64, 151 * 64]] {
                for shape in 0..4 {
                    let mut contours = match shape {
                        0 => vec![vec![[0, 0], [24576, 0], [24576, 18432], [0, 18432]]],
                        1 => vec![
                            vec![[0, 0], [16384, 123], [16311, 16384], [17, 16311]],
                            vec![[4000, 4000], [4000, 12000], [12000, 12000], [12000, 4000]],
                        ],
                        2 => vec![
                            vec![[0, 0], [16000, 15000], [64, 14000], [15000, 51]],
                            vec![[2000, 1000], [18000, 1000], [18000, 13000], [2000, 13000]],
                        ],
                        _ => vec![vec![
                            [0, 0],
                            [8000, 1000],
                            [3000, 14000],
                            [16000, 7000],
                            [1000, 5000],
                            [17000, 16000],
                        ]],
                    };
                    for contour in &mut contours {
                        for p in contour {
                            p[0] += offset[0] + phase;
                            p[1] += offset[1] + phase;
                        }
                    }
                    let stats = check(&contours);
                    kinds.empty_regions += stats.empty_regions;
                    kinds.solid_regions += stats.solid_regions;
                    kinds.halfplane_tiles += stats.halfplane_tiles;
                    kinds.complex_tiles += stats.complex_tiles;
                }
            }
        }
        assert!(kinds.empty_regions > 0 && kinds.solid_regions > 0);
        assert!(kinds.halfplane_tiles > 0 && kinds.complex_tiles > 0);
    }

    #[test]
    fn integrated_curves_keep_legacy_pixels_witnesses_and_rejections() {
        for phase in [0, 1, 7, 31, 63] {
            for offset in [[0, 0], [-7777, -3151], [19001, 7123]] {
                let p = |x: i64, y: i64| point([x + offset[0] + phase, y + offset[1] + phase]);
                let ops = [
                    PathOp::MoveTo(p(0, 0)),
                    PathOp::CubicTo(p(3000, -2000), p(13000, 16000), p(16000, 7000)),
                    PathOp::QuadTo(p(21000, 20000), p(2000, 16000)),
                    PathOp::LineTo(p(9000, 7000)),
                    PathOp::Close,
                ];
                for (memory, work) in [
                    (512 * 1024 * 1024, 32_000_000),
                    (4096, 1_000_000),
                    (16384, 1_000_000),
                    (131072, 1_000_000),
                    (1048576, 64),
                    (1048576, 1024),
                ] {
                    let limits = Limits::default()
                        .with_transient_bytes(memory)
                        .unwrap()
                        .with_path_segments_per_frame(work)
                        .unwrap();
                    let mut old = crate::timing::StageTimings::default();
                    let mut new = crate::timing::StageTimings::default();
                    let a = crate::axis_widen::rasterize_legacy_body_timed(&ops, &limits, &mut old);
                    let b = crate::axis_widen::rasterize_body_timed(&ops, &limits, &mut new);
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            assert_eq!(
                                (a.bounds(), a.stride(), a.is_xy_coverage()),
                                (b.bounds(), b.stride(), b.is_xy_coverage())
                            );
                            assert_eq!(a.data(), b.data());
                            assert_eq!(
                                (old.raster_peak_bytes, old.raster_required_work),
                                (new.raster_peak_bytes, new.raster_required_work)
                            );
                            if memory == 512 * 1024 * 1024 {
                                assert_eq!(new.tiled_fills, 1);
                            }
                        }
                        (Err(a), Err(b)) => assert_eq!(
                            (a.code(), a.resource_failure(), a.detail()),
                            (b.code(), b.resource_failure(), b.detail())
                        ),
                        _ => panic!("tiled/legacy curve result differs"),
                    }
                }
            }
        }
    }

    #[test]
    fn unproved_extents_and_insufficient_optional_scratch_decline() {
        let segments = [Segment::new([0, 0], [1280, 1280]).unwrap()];
        let bounds = PixelRect::new(0, 0, 24, 24).unwrap();
        assert!(rasterize(&segments, bounds, 1, 0).is_none());
        assert!(rasterize(&segments, PixelRect::new(0, 0, 1, 1).unwrap(), 1_000_000, 0).is_none());
        // Open contours cannot silently paint to the end of the tile.
        assert!(rasterize(&segments, bounds, 1_000_000, 0).is_none());
    }
}
