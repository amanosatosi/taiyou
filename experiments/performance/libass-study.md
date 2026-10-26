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
