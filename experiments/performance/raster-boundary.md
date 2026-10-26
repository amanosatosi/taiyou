# Exact raster substitution: measured boundary, no new public framework

Current production reference9265bb3b90b7c17ce07cbf84d60b88ae40ec578f. Retain
row traversal, row partition and revised publication. MT and worker policy freeze.
The direct-core baseline in results/mangetsu-core shows changing4K stress386.79ms
vs pinned Mangetsu RGBA24.87ms; prior post-sample profiling assigns242ms to span
union. New coarse profiling precedes raster substitution; no speed claim yet.

## Source mechanism and semantic boundary

The supplied local libass files match the pinned commit
f61db567e6593df3470e91594bcd4ad2d0473aff after CRLF normalization, verified against
the pinned GitHub file blob hashes (not an inferred archive commit):

| File under libass/ | Pinned Git blob |
|---|---|
|ass_rasterizer.c|ab7fddb5f91eb2047ee509309316ccccbe13803b|
|ass_bitmap.c|41df5e1ac95e4cff4feca4ce0129d45d2ce9a57e|
|ass_cache.c|47069c15a7d9407bbe6897d9da2f8586d955e5e0|
|ass_render.c|18f856a8eae43390871d2cc6ea6920a3c7fc303d|
|ass_bitmap_engine.c|847adc4fb56805998f2042fc0cce30ed002c997f|

The tile template is libass/c/rasterizer_template.h, pinned blob
2792f0b22f0f163f437f8dc19d18f47053832950. ass_rasterizer.c retains segments in two
reused buffers, partitions them spatially, propagates winding and selects solid,
half-plane or generic tile kernels. The template's analytic area approximation
and A8 arithmetic are NOT Taiyou's native xy eighth-grid coverage. Bitmap-engine
dispatch and run/composite caching are downstream references, not permission to
replace that arithmetic or copy libass's thread ownership.

The smallest candidate boundary is inside axis_widen.rs after the existing
relative-origin curve flattening, before line() expands each straight segment
into one Edge per native eighth row. Preserve the exact flattened endpoints,
quadratic-to-cubic conversion, polynomial truncation and final endpoint closure.
Retain segment direction, native row interval and the existing16.16 crossing rule.
Do not move Q6 quantization, origin alignment, phase or transform evaluation.

First slice: ordinary closed fill outlines, rx=ry=0. Retain immutable flattened
segments; recursively partition their spatial influence into bounded tiles.
Carry nonzero winding, cheaply clear/fill constant tiles, and evaluate native
eighth-grid samples with the existing crossing rule for half-plane/complex leaves.
Local leaf sorting is permitted; the giant global edge/span/expanded-span sort is
the work to remove. Do not call the old full producer on every successful tiled
render and call that an optimization. Open/unsupported geometry and borders keep
the exact original producer until individually proved. The first test-only tile
slice and its resource witness proof pass (results/tile-fill-proof and
results/tile-resource-proof). The next candidate integrates it only at the native
body-fill entry. Curves share the unchanged relative-coordinate polynomial and
endpoint emission with the retained producer. The legacy body remains a direct
test oracle. Expand only after exact output and same-runner measurements; no
production speed claim follows from the test-only proofs.

The output remains the existing immutable native0..64 Mask with the same bounds,
padding and coverage units. Paint/clip/filter/composition/cache keys stay outside
this boundary. Nonzero fill, self-intersections, glyph/drawing contour grouping,
transforms and clips require byte equality, not analytic similarity. libass's
outline stroke cannot replace Taiyou's discrete ellipse widening as a shortcut.

## Resource and ownership constraints

Replacing real allocations is allowed; accepting a formerly rejected request is
not. Preserve legacy work/capacity witnesses and error ordering with explicit
preflight accounting, including flattened segment count, crossing/interval work,
native mask extent, transient peak and cache-hit rechecks. A narrow fill candidate
must retain the old path if equivalent preflight cannot yet be proved. New scratch
must fit existing limits and be renderer-owned or request-local, never global.
Retained mask/frame Arcs cannot point into reused scratch. No native shared face,
new worker pool, final-frame cache or benchmark-only semantic path.

## Profiling interpretation

Keep primary samples timer-disabled. Supplemental31-frame changing profiles use
the same further16ms timestamps for candidate/reference and compare RGBA and
diagnostic SHA-256 after rendering. Separate segment generation, edge sorting,
span generation, row bucketing, interval sorting and merging. Track explicit Vec
allocation/reallocation only at capacity growth, with cumulative capacity bytes.
Allocation time overlaps its owning stage. Bitmap clearing and publication are
separate; coverage arithmetic and destination accumulation are one fused loop,
reported together with write sample count rather than inventing exclusive times.
These observations do not instrument libass/Mangetsu internals or change fixtures.

Every kept slice needs same-runner before/after Taiyou1T and pinned direct stock,
Mangetsu legacy and RGBA API times, cache OFF/ON, default-resource equality and the
full historical digest audit at a promising checkpoint. Probe gates stay failing.
