# Optimization source observations

Read-only local reference: `C:/taiyou-time/refrences/libass-master`, archive
RELEASEVERSION 0.17.5, without Git metadata. This is the supplied current local
source, not a claim to know today's upstream HEAD. The reference fingerprint
check passed for all 24 recorded entry points. No source was fetched or replaced.
The locked external oracle is unchanged. libass is an optimization reference;
Taiyou's existing pixels and ownership/determinism contracts remain authoritative.

| Source boundary | Work avoided / complete dependency principle |
|---|---|
| `ass_cache_template.h:54`, font key | Family, weight, italic and vertical selection; retain the selected font object |
| `:72`, face-size metrics | Face and exact size, independent of glyph and paint |
| `:79`, glyph metrics | Face/size/glyph metrics reusable by subsequent layout |
| `:88`, glyph outline | Font, face, size, glyph, synthetic bold/italic and decorations |
| `:101`, drawing outline | Retain owned drawing text, rather than an ephemeral view |
| `:107`, border outline | Underlying outline plus stroke scales and border axes |
| `:62`, bitmap | Retained outline plus transform and subpixel offset; excludes solid paint |
| `:117`, filter and `:127`, bitmap references | Filter parameters and complete ordered bitmap placements |
| `ass_render.c:2587,2643`, composite | Reuse combined/filter results before final paint/composition |
| `ass_cache.c:477,488`, reference release/cut | Eviction and final release differ; pinned dependencies retain their lifetime/charge |

`ass_render.c:138..143` constructs distinct font, outline, face-size metric,
glyph-metric, bitmap and composite stores. `:3021` cuts composite, bitmap and
outline stores against their respective limits. This is not a single shaping
cache. The useful transferable principles are layered immutable results, exact
dependency keys, retaining prerequisite ownership, and bounded eviction/accounting.

`ass_shaper.c:213..274` shows why foundational metrics are separate from complete
shaped text: face-size construction sets size and copies metrics; glyph-metric
construction sets size and loads the glyph. `:466..484` supplies these cached
metrics to a HarfBuzz sub-font. These avoid repeated native metric work across
otherwise different text runs. The native font faces involved are mutable; this
is not permission to share them across Taiyou workers. Taiyou would retain owned
immutable metric/outline values through its own validated boundary when justified.
The renderer's limit-cut function visibly handles composite/bitmap/outline stores;
do not infer an identical bound for every other libass store or adopt negative
native-error sentinels as Taiyou's error-retention policy.

Taiyou must not copy libass's transform quantization, coverage, blur, outline
semantics, pointer-derived keys or threading limitations. Its source-backed
mask arithmetic remains unchanged. Foundational metric/outline reuse is still a
candidate when measurements or downstream dependencies justify it; today's small
shaping time is not a permanent architectural assumption.

Additional inspected file hashes (SHA-256):

- `libass/ass_cache.c`: `a9cb10ec2a4cec63ac3c2140f48e06fc5d2f0d4e1ccf0362c6483d2171c84c7f`
- `libass/ass_cache_template.h`: `5f388b50434e64da69f5f05384709e0797025fa708e3676cfa0893e41c6cf8e9`

These are file fingerprints, not a full-tree or commit authentication. The local
source is ISC-licensed. This note records studied techniques; no libass code has
been copied into Taiyou.

## Serial composition investigation after the controlled baseline

The final same-runner controlled dataset is results/libass-style-selected. Warm
composition dominates Taiyou's prior stage profile; changing geometry retains a
large span sorting/union cost. Local test/test.c:139..164 traverses each bitmap
and destination row with invariant strides, rather than revalidating a full
coordinate lookup per pixel. This simple proven traversal is the first candidate;
Taiyou retains its own xy native-coverage and A8 integer blend arithmetic.

ass_render.c:2447..2757 combines reusable glyph bitmaps and caches filter/composite
results before final output. Its final ASS_Image list is not a whole RGBA frame.
ass_detect_change at3310 compares retained image lists; the paired benchmark still
composes a fresh RGBA output even for changed=0. Taiyou must not claim its frame
API is equivalent to returning an unchanged image list without that output work.
No renderer semantics, cache topology or scheduler change is justified merely
by these observations; measure the narrow traversal first.

The row traversal probe is retained in results/serial-row-traversal: warm stress
157.38->60.09ms, changing stress452.14->350.32ms, exact output/diagnostics preserved.
Remaining changing-frame cost is consistent with the already measured large
span sorting/union stage. ass_rasterizer.c:630..716 partitions bounded regions
before local raster work rather than globally sorting expanded border spans.
Its analytic coverage and outline-border semantics cannot replace Taiyou's xy
eighth-grid algorithm without changing pixels. Adapt the locality principle
narrowly: partition the existing exact spans by integer y in place, then sort
left/right within each row. Keep the original union and raster accumulation.
Optional two-array row scratch fits within the existing producer peak witness;
decline to the old sort on insufficient budget, sparse/extreme range or failed
reservation. Candidate counts, required work and resource witness stay unchanged.
No libass source code is copied; this is ordinary counting partition/sorting.

Additional inspected file SHA-256:

- test/test.c: 3babcee23a5d8fa8a796f4e2ae49e68faa2b1ba892f20b6637d386b680401fbf
- libass/ass_rasterizer.c: dc3c3049b1322b63795055144d2b7e7c66aa86e2da50316dde39a6ee39fc590f

The row partition probe reduces changing stress447.06->407.45ms and transforms
79.27->67.67ms, but remaining span union is still242ms in its post-sample stress
observation. The same run isolates Frame publication at13.91ms of a21.96ms warm
4K dialogue frame. Taiyou validates every premultiplied pixel and copies a Vec
into an inline Arc slice; libass's image API retains referenced immutable bitmap
storage. The measured publication candidate retains every public validation and
the existing output lease/capacity checks, but shares a boxed owned slice through
Arc, avoiding that pixel-sized copy. Box length equals charged length, unlike
retaining a spare-capacity Vec. All old conservative composition reservations
remain unchanged. The first branchless full-buffer RGB<=alpha reduction passed
correctness but had mixed performance, including a border warm regression; it is
rejected as-is (results/serial-publication). The next candidate checks every byte
in all-zero blocks using portable word ORs; zero RGBA is necessarily valid. Every
nonzero block and tail retains the original exact channel validator. There is no
trusted-producer bypass, unsafe SIMD, weakened pixel assertion or new interface.

## After removing expanded-span storage and sorting

results/tile-border-local records the paired changing4K reduction377.12->139.50ms.
Its post-sample stress profile now has18.5ms tiled borders,54.7ms filters,45.7ms
composition and12.8ms base-outline intake. The allocation counter is~0.08ms, so
a new scratch-pool framework is not justified by this measurement. Existing
Taiyou raster.rs:coverage_groups already joins compatible text before raster and
filter work; drawing words deliberately remain separate to preserve seams and
translucent overlaps. Do not add a second run-grouping system without evidence.

Additional local files match stock pin f61db567e6593df3470e91594bcd4ad2d0473aff,
verified by normalized Git blob identity against the pinned repository file:

|File|Git blob|
|---|---|
|libass/ass_blur.c|ec91a92410d67ac7da426f373309720c1b7a8e4f|
|libass/c/blur_template.h|7597b48a88be1ca446a88d38454e70d316d27b93|

ass_blur.c's cascade downsamples, filters and upsamples using approximate kernels.
That coefficient/rounding scheme cannot replace Taiyou's exact existing Gaussian.
The C blur template instead offers a useful data-layout principle: accumulate
multiple adjacent samples using contiguous local storage. Current Taiyou loops
recompute coordinates/bounds for every pixel/tap and gather vertical samples.
The narrow candidate is loop interchange with contiguous rows, preserving each
pixel's original f32 tap order (including native0,2,1,3 groups), intermediate
precision, coefficients and16-column/tail conversion. One reusable float row can
fit the unchanged preflight allowance for two float planes. This is an unmeasured
candidate direction, not a speed claim, adopted kernel approximation or SIMD plan.
Production filter code remains unchanged at the tiled checkpoint.
