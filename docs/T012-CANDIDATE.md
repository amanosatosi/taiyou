# T012 scalar Core fill raster candidate

Base: merged T011 `83894682eb2d33affd7175eb90cfdd502b32bdb4`.
Keep unmerged for shared-interface, native/FFI and CI technical review. Candidate
checks are not independently trusted validation or completion of the T005 gate.

## Backend and ownership

FreeType 2.14.3, pinned by the unchanged T002 recipe, provides `smooth/ftgrays`.
Raster policy revision 1 calls `FT_Outline_Render` with AA|DIRECT|CLIP (7), an
outline flags value of zero (nonzero winding), and 256 coverage levels. No LCD,
SDF, hinting, embedded bitmap, gamma conversion or custom scan conversion is used.
The [pinned span ABI](https://github.com/freetype/freetype/blob/VER-2-14-3/include/freetype/ftimage.h)
uses **unsigned** 16-bit x and length; the old binding's span type is not reused.
C header assertions and Rust layout tests check the adapter structures.

`RasterContext` owns the existing private per-context native library owner and
is neither Send nor Sync. Callbacks, pointers and native arrays stay exclusively
in `taiyou-font/src/native.rs`. A synchronous span callback validates counts,
scanlines, ordering, extents and duplicate writes before copying into a reserved,
zeroed Rust buffer. Callback failure marks the call invalid and publishes no mask.
The pinned backend emits batches of up to 16 spans and may include a valid
zero-length clip-edge tail; such empty spans perform no writes. Malformed counts,
nonempty out-of-bounds spans, null buffers and duplicate nonzero writes still fail.
No callback allocates or unwinds. Published Mask data is immutable and outlives
all native contexts and temporary outline buffers. Existing C5 Mask validation
checks exact extent/stride/length and zero padding; no color enters an A8 value.

## Coordinates and groups

T011 residual device commands already contain their phase. Add the integer pixel
offset times 64 exactly once; do not add phase again or re-quantize floats. Bounds
floor minima and ceil maxima using signed Euclidean arithmetic. They contain the
entire control hull, including negative/offscreen support; no viewport clipping
is performed. For the native call, translate to the mask-local hull and reflect
y-down into y-up about its bottom edge. Native scanline y maps to row height-1-y.
The CLIP box is that full source allocation, not the visible viewport.

`coverage_groups` traverses T011 visual glyph order and forms maximal adjacent
groups with equal ShapeStyle, GeometryStyle, EffectStyle and PaintSet. C3 already
classifies shape changes as geometry changes. Fallback face changes under the
same evaluated request stay in the group. A paint boundary inside one shaped
cluster retains C4's first-logical-scalar association. Groups retain ordered
instance ordinals, profile/revision and the event/source owner. Native rasterization
combines all group contours in one nonzero-winding call, never by adding separately
rasterized A8 glyphs. Explicit host/synthetic groups declare compatibility; only
CoreV1 Fill is accepted. Border/Shadow/Legacy execution remains unsupported.

## Bounds and limits

Preflight checked counts, dimensions, stride products, pixels, output bytes,
combined commands, owned buffers and a conservative native scratch reservation.
The reviewed binding limits a single unsplit winding group to 32767 points and
contours. The pinned span ABI limits mask width/height to 65535 in addition to the
caller's smaller dimension/pixel/byte limits. Exceeding either returns LimitExceeded;
splitting the group would change winding and is not a workaround. Native scratch
estimates are not a hard process RSS guarantee (C7/R003). No cache or worker pool
is added. A future frame coordinator must reserve aggregate frame intermediates.

Empty input, zero-area source bounds and all-zero native coverage publish canonical
zero-dimensional, zero-stride masks. Other results retain conservative support,
not a fabricated advance rectangle or a viewport crop.

## Evidence

Synthetic rectangles use independently calculated pixel intersection areas and
the pinned 256-level grayscale quantization (full coverage saturates to 255).
Tests cover winding/holes, overlap, fractional and negative placement, limits,
invalid native spans, padding and native lifetime. Fixed-font inputs reuse the
four existing pinned/hashed licensed fixtures, not new or regenerated goldens.

Actions archive sample.a8, input.ass, report.json and a lossless coverage.pgm
diagnostic preview. The report records candidate SHA, font/source revisions and
hashes, backend/flags, signed bounds, dimensions, stride and actual A8 SHA-256.
CI uses MIT-licensed actions/upload-artifact v4.6.2 at
`ea165f8d65b6e75b540449e92b4886f43607fa02` for seven-day stage artifacts.
External storage requiring additional credentials was unnecessary; no Rust or
native dependency was added. Workflow additions remain subject to CI review.
On Linux an independently authored C test program uses FT_Outline_Get_Bitmap over
the same checked Q6 stage commands, independently computes bounds and top-down
bitmap storage, and compares exact bytes against the direct-callback Rust adapter.
Both Latin CFF and the four-font CFF/TrueType fallback scene receive this check.
Expected/actual/difference PGM previews and zero-tolerance V3 metrics are archived.
The C test is a candidate-side API cross-check, not independently trusted T005
validation or a promoted font golden. Its rectangle probe independently reproduces
the synthetic coverage expectations before Taiyou's raster tests run.
This is a genuine stage sample, not an approved compatibility baseline. The native
matrix checks sample identity again after full/MSRV runs; differences fail.
No local compilation or artifact download is required. No performance or
VSFilter parity claim; no frame, PNG, border, blur or shadow implementation.
