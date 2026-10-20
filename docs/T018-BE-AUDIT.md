# T018 BE review: input geometry and coverage prerequisite

This is a corrective review record, not a new rendering policy or an assertion
that the four BE images pass. All four frozen `be.ass` frames render in strict
mode with exit 0 and **differ at pixel comparison**. Their frozen inventory label
does not explain or excuse these differences. No production filter, font metric,
line-height, shadow or raster policy was changed during this audit.

## Reproduction and provenance

Reviewed production: `f70ec7e4486156012ccd36dd2652e4419b8f03de`.
Base: `5c6672660b50a69a98518a2fc024abd27559807d`.
The isolated diagnostic revision is
`b92c16060345f225f285531468f1895e7d9f2846`; its production tree is unchanged.
[Focused Actions job](https://github.com/amanosatosi/taiyou/actions/runs/39444294580/job/118701991703)
ran source tests, focused native rendering, exactly the four frozen comparisons,
the real ASS stage reconstruction and controlled diagnostic copies.
[Original artifact](https://github.com/amanosatosi/taiyou/actions/runs/39444294580/artifacts/12274099751)
ZIP SHA-256: `39da906e057daf35fafebd1387332ea23c640272e4ea25ac067ee2a9123ecd1a`.
It contains the unmodified comparator report/differences, raw RGBA, before/after
A8 masks, bounds/stride metadata, decoded glyph groups, line metrics, control
scripts/images, font metadata and complete numerical measurements.

Inputs are the pinned libass-tests corpus from the existing acquisition path;
no new oracle or substituted font was used. `be.ass` SHA-256:
`f3033c3d95abe212f52cfaa3a5b96b1c557adf6f826f13405094a4877076c5dc`.
Resolved Aileron-Regular.otf SHA-256:
`2762f4fc2ebad8323264aea52ffa2260b86c9677493d3ce2dc4f34e5851d2aa2`.
The job uses the existing pinned native recipes and requirements on Ubuntu24.04.
Frames are 800x600, CoreV1, one worker, strict policy, timestamps 500000,
1500000, 2500000 and 3500000 microseconds. Raw output is premultiplied RGBA8.
Comparator metrics below use its unchanged premultiplied encoded RGBA16 domain.
All bounding boxes are half-open; pixel counts count nonzero alpha.

## Measured four-case result

No production correction is justified within BE alone. Consequently before and
after production pixel counts remain identical; **none is exact**.

| Case suffix | Differing pixels, before → after | Expected alpha bounds | Actual alpha bounds | Expected nonzero | Actual nonzero |
|---|---:|---|---|---:|---:|
| be-0500.png | 25,098 → 25,098 | [146,168,655,493] | [96,85,703,588] | 11,799 | 13,858 |
| be-1500.png | 30,200 → 30,200 | [144,167,656,494] | [96,85,703,588] | 14,606 | 16,313 |
| be-2500.png | 32,679 → 32,679 | [143,167,656,494] | [95,84,704,589] | 15,236 | 18,261 |
| be-3500.png | 33,214 → 33,214 | [143,167,656,494] | [95,84,704,589] | 15,389 | 18,673 |

Raw RGBA SHA-256, in the same order:

```text
5b5779a06c346fe1806fb7e3f4c546f1e1223a6ab49784ecccd6201a2dde5276
108b9f7b7e00d703fbea4109535db72e857926e2b8c36b83075092d56f1b13e1
3b115bfa7626ac04eb7c284510f8963a9e7ca5218d665cf51853e3df62504c42
03cc522ea8a8d0e51fddc502ff74c0ac057b54d1d3b7439fd48ee1c98e6fe6a7
```

The dominant initial differences are whole-glyph size and inter-line position,
not just halo rounding: at 500ms expected occupied row bands are [168,185),
[282,327), [420,493), whereas actual bands are [85,99), [282,329), [508,588).
Only 555 alpha-positive pixels intersect at this timestamp; 13,303 occur only
in actual and 11,244 only in expected. The centered middle row is close in y;
the first and third rows are displaced in opposite directions.

## Source-to-output attribution

The read-only pinterf archive is pfmod3.2.0.810 from LOCAL_REFERENCES. Its exact
equivalence to locked commit `25cf3e8433b3a83a5489e47a2c335516492bb815` remains
unverified. File hashes are in [T018 candidate](T018-CANDIDATE.md); no source was
refetched, translated or modified. These are source observations and measured
Taiyou controls, **not authenticated xy intermediate-mask captures**.

1. **Font-size interpretation precedes BE.** RTS.cpp CMyFont116–137 uses positive
   LOGFONT height and GetTextMetrics; event creation2957 scales that height by64.
   CText::CreatePath644–695 obtains GDI outlines. Taiyou's shape.rs563/904 and
   geometry.rs315 instead map one em to font size, as normative C4 requires.
   The actual font has UPEM1000, Windows ascent970/descent230 (also hhea970/-230).
   Its source-guided cell-to-em control is therefore 1000/1200, not a guessed
   per-fixture constant. With that factor alone (and shadow disabled), actual
   x bounds move from [96,703) to [147,652) at 500ms; expected is [146,655).
   This control supports the metric explanation; it is not a GDI capture or
   proof that this simple ratio reproduces every GDI font/rounding choice.
2. **Blank-line layout precedes BE.** GetNextLine1453–1472 halves ascent/descent
   for an empty line. Core C4/T010 intentionally preserves full style metrics.
   The fixture contains doubled mandatory breaks between its three glyph rows.
   A separate owned control makes only the blank lines half-sized through
   existing font-size overrides. Together with the font-size control it moves
   actual row bands to [171,182), [285,324), [423,490) at 500ms. BE is unchanged.
3. **Coincident shadow changes opacity.** RTS.cpp1243–1256 suppresses a shadow
   when both offsets are zero. This fixture's shadow paint is opaque black with
   zero offsets. Core's existing protected rendering tests deliberately retain
   visible zero-offset shadows. The new real-fixture test reconstructs the
   frame from two identical black draws per group and proves byte equality with
   Renderer::render. A single-fill reconstruction is exactly equal to the CLI
   control that changes only shadow opacity to transparent. This isolates the
   extra opacity independently of the font/layout controls.

Controls are clearly labeled modified input copies in the diagnostic output;
the original corpus and production path remain untouched. Their combined
contribution is measured, not a sum of independently additive errors:

| Frame | Original alpha MAE | Shadow hidden | + source-guided cell size | + half blank lines | Combined differing pixels |
|---|---:|---:|---:|---:|---:|
| 0500 | 2216.428502 | 2040.553229 | 1210.414852 | 39.228373 | 8,817 |
| 1500 | 2210.000290 | 1940.646085 | 1160.355000 | 72.800604 | 13,964 |
| 2500 | 2235.864663 | 1891.367942 | 1133.892031 | 76.264215 | 15,572 |
| 3500 | 2172.065483 | 1792.374754 | 1086.900117 | 106.360521 | 16,007 |

Combined alpha error falls by approximately 95–98%, with unchanged BE equations.
This does **not** make a control an exact frame or an accepted compatibility
policy. The additional no-BE control retains the normalized glyph envelope
[147,171,652,490) at every timestamp. Significant pre-filter geometry differences
are therefore observable before filter strength is varied.

## Coverage and BE arithmetic audit

| Item | pinterf location/behavior | Taiyou audit outcome |
|---|---|---|
| Input body | Rasterizer::Rasterize788–882 accumulates 8x8 subpixel spans into byte coverage0..64 | FreeType unhinted grayscale A8 is0..255; it is a different rasterizer/input unit, not evidence of equal xy masks |
| Input rectangle/origin | Rasterize788–835: eighth-pixel offsets, visible width `ceil(width/8)+1`, analogous height; pitch rounded to16 | coverage_bounds uses tight floor/ceil of Q6 geometry; no equivalent trailing row/column. BE's retained edge depends on this input extent |
| Padding | BeBlur1167–1187: per-axis `round(float(target)*sqrt(float(be)/4))` | BlurSpec uses the same source arithmetic; BE1/2/4 pad1, BE8/16 pad1/2 respectively; no N-pixel support or added guard ring |
| Visible width versus pitch | BeBlur1187–1226 copies visible rows and passes visible width/height separately from aligned pitch | Rust copies only visible width using input stride; packed output stride equals visible width. New pitch test verifies identical input pixels with stride5 versus16 give identical results |
| Scaled strength | BeBlur1230–1233: float `be*0.5*(target_x+target_y)`, truncate integer pass count | Same float arithmetic and integer conversion. Target=(1,1); fixture BE1..16 has no fractional remainder |
| Integer passes | be_blur568–671, BeBlur1234–1242: 3x3 [1,2,1] outer product, one truncating /16, unchanged outer allocation ring | Exact comparison with an independently expressed separable integer equation passes for every actual nonempty coverage group at BE1/2/4/8/16. No horizontal-axis truncation or boundary filtering is inserted |
| Fractional coefficient/order | xy_filter.cpp1248–1292 and1340–1387: coefficient from residual scaled float, horizontal then vertical, actual scalar XyRounding expression | Existing integer/fractional/order tests preserved. Rust uses the same scalar expression; this fixture does not exercise fractional/vector-tail equivalence |
| Output alpha | Rasterizer.cpp FillAlphaMashBody_c2330 onward: `(coverage*paint_alpha+32)>>6` | Core C6 uses A8 mul255 and source-over. Changing input units after BE is not equivalent to filtering xy units first |

The added source-unit counterexample makes the last issue executable: a full
coverage impulse filtered once has center16 in xy units, converted to alpha64;
an A8 impulse has center63 after the same /16 equation. Quantization and BE do
not commute. Dividing FreeType samples by four cannot reconstruct pinterf's
8x8 scan conversion, GDI outline or eighth-pixel phase, and is not a valid fix.

The actual-stage test records `.`, `+`, `¶` and their source groups before BE,
after BE1/2/4/8/16, bounds/stride, and each event's actual strength. Reconstruction
must equal the real Renderer and existing comparator raw hash. Thus it tests the
real `be.ass` path rather than only an impulse. Integer arithmetic is verified
for identical bytes and rectangles; **the complete xy BE pipeline is not verified
pixel-identical**, because its input units/rectangle/raster/placement differ.
Residual differences after the controls cannot be uniquely divided between GDI
outlines, subpixel scan conversion, coverage quantization and overlay allocation
without authenticated intermediate xy masks. No fabricated decomposition is used.

## Stop boundary and concrete remaining work

Exact output requires a consistent compatibility path for GDI cell-height
metrics, half-height empty lines, zero-offset shadow suppression, and native
xy overlay origin/extent/0..64 coverage before filtering and alpha conversion.
Those are materially separate font/layout/raster/composition policies, including
direct conflicts with current Core C4/C6 and protected zero-offset-shadow tests.
This review therefore stops that implementation portion under AGENTS rule3 and
the owner's explicit separate-prerequisite boundary. It does not change Core
contracts, existing tests, or distort BE to cancel upstream errors.

Minimum next evidence: an authenticated pinterf render/stage-mask capture of
this exact font/script at the four timestamps, with font/GDI metrics, eighth-pixel
origins and pre/post-BE0/1/2/4/8/16 overlay bytes/pitches. That can discriminate
the residual raster/allocation contribution. A maintainer decision is then
needed on the compatibility metric/empty-line/shadow policy scope before changing
the existing Core guarantees. The source audit already identifies the concrete
implementation dependency; another blur kernel design or tolerance is unnecessary.

The final PR report records exact-head normal platform/MSRV/sanitizer and one
complete external comparison. No expected PNG, frozen asset, inventory label,
tolerance, comparator algorithm, error mapping or acceptance policy is modified.
PR32 remains unmerged and these four compatibility failures remain open.
