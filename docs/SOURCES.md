# Source study and provenance

## S0. Local-first access

The user has now supplied source folders in `C:\taiyou-time\refrences`. Use [LOCAL_REFERENCES.md](LOCAL_REFERENCES.md) and [local-sources.json](local-sources.json) for local file links and fingerprints. This supersedes the initial availability statements below: Guliverkli2 historical source is now present; builds/captures and exact archive-to-commit equivalence remain unverified. The online links below preserve the initial study's provenance and are not instructions to browse or fetch again.

## S1. Inspection record

Initial study on 2026-10-17: the supplied workspace was empty, not initialized as Git, and no local VSFilter checkout was then available. **pinterf/xy-VSFilter** remains the sole main xy reference, per user direction. Do not substitute Cyberbeing, MPC-HC, XySubFilter or VSFilterMod and label the result “xy”. The later-supplied Guliverkli2 archive is separate historical evidence; see S0 for current availability.

Exact revisions were resolved through GitHub's commits API. Selected files were read from those revisions into temporary study storage, not copied into Taiyou. [references.lock.json](references.lock.json) is the machine-readable pin record. Web-indexed master pages were useful for discovery, but are not immutable test inputs. No reference renderer was built or executed during this assignment; no pixel measurements or performance results are claimed.

| Reference | Pinned revision | Scope actually inspected |
|---|---|---|
| [libass](https://github.com/libass/libass/tree/f61db567e6593df3470e91594bcd4ad2d0473aff) | f61db567e6593df3470e91594bcd4ad2d0473aff | Pinned cache template and bitmap engine; indexed render/shaper/COPYING; repository structure |
| [libass-tests](https://github.com/libass/libass-tests/tree/a5073319af8117271d8ac13ef5c088197ff2d59b) | a5073319af8117271d8ac13ef5c088197ff2d59b | Root documentation, pinned regression README; corpus not downloaded/executed |
| [libassmod](https://github.com/amanosatosi/libassmod/tree/3f5640045bbd737661f3858480ec948fece929cb) | 3f5640045bbd737661f3858480ec948fece929cb | Pinned COPYING, RGBA source allocation/API portion; indexed README/RGBA documentation |
| [pinterf/xy-VSFilter](https://github.com/pinterf/xy-VSFilter/tree/25cf3e8433b3a83a5489e47a2c335516492bb815) | 25cf3e8433b3a83a5489e47a2c335516492bb815 | Pinned License.txt, selected RTS.cpp geometry/paint code and Rasterizer.cpp entry points |

## S2. Engineering observations and implications

These are limited source observations, not evidence that Taiyou matches the implementation.

* libass's [cache descriptors](https://github.com/libass/libass/blob/f61db567e6593df3470e91594bcd4ad2d0473aff/libass/ass_cache_template.h) separate font, glyph/drawing outline, transformed bitmap, border and filter data. This supports using distinct artifact caches; Taiyou does not adopt its pointer-based key scheme.
* libass's [bitmap engine](https://github.com/libass/libass/blob/f61db567e6593df3470e91594bcd4ad2d0473aff/libass/ass_bitmap_engine.c) dispatches scalar/x86/ARM kernels. Taiyou adopts a scalar oracle and measured dispatch, not its assembly or internal architecture.
* The inspected [shaper](https://github.com/libass/libass/blob/master/libass/ass_shaper.c) uses HarfBuzz and FriBidi, explicit run segmentation and spacing-sensitive ligature settings. The [render code](https://github.com/libass/libass/blob/master/libass/ass_render.c) includes font/outline/bitmap caches, collision metrics and transformed outlines. These indexed views must be rechecked at the pin before implementation relies on a particular expression.
* The pinned [xy RTS path](https://github.com/pinterf/xy-VSFilter/blob/25cf3e8433b3a83a5489e47a2c335516492bb815/src/subtitles/RTS.cpp) transforms path data before scan conversion/widening, uses separate unblurred overlays, and calls GDI for text measurements. Its paint code notes distinct shadow alpha rounding. These motivate explicit stroke-order, metric and alpha-policy experiments rather than claiming FreeType equivalence.
* The pinned [xy Rasterizer](https://github.com/pinterf/xy-VSFilter/blob/25cf3e8433b3a83a5489e47a2c335516492bb815/src/subtitles/Rasterizer.cpp) exposes separate Gaussian/be blur paths, scan conversion and widened regions. Kernel arithmetic and quirks are implemented from the source path as features reach the renderer.
* [libassmod's README](https://github.com/amanosatosi/libassmod/blob/master/README.md) describes directional blur, distortion, movement, gradients and host-supplied image fills, while warning that behavior is experimental. Its [RGBA implementation](https://github.com/amanosatosi/libassmod/blob/3f5640045bbd737661f3858480ec948fece929cb/libass/ass_render_rgba.c) explicitly bounds allocations. Taiyou therefore keeps paint and host resources separate; it does not promise these extensions or treat this fork as a normative Mangetsu specification.

The upstream [regression instructions](https://github.com/libass/libass-tests/blob/a5073319af8117271d8ac13ef5c088197ff2d59b/regression/README.md) describe per-test fonts, expected PNGs and knobs that regenerate, skip or relax tests. Taiyou's trusted runner must forbid these knobs. Upstream [crash tests](https://github.com/libass/libass-tests/blob/a5073319af8117271d8ac13ef5c088197ff2d59b/README.md) are distinct from image regression and use system fonts; Taiyou must record/fix that environment independently.

## S3. Still to inspect

For ordinary ASS/SSA compatibility, inspect the local pinterf source directly and implement the observed behavior. Useful source-verified observations retained from the former research notes include: ordered paint assignment; first-position/first-alignment precedence; integer-millisecond transform evaluation; positive GDI cell-height sizing; q-mode word wrapping followed by CLine compaction; nonzero-winding scan conversion on the historical eighth-pixel grid; Gaussian-before-BE filtering; source-defined BE padding/truncation; and separate shadow/border/fill handling. These are source observations, not automatic pixel-identity claims.

R004 is reserved only for transformation details that remain ambiguous after source inspection, such as unresolved projective/aspect edge cases. T015 must inspect upstream `compare` filename/timestamp/scale conventions rather than assume one screenshot per script. T031 must map the actual Mangetsu source/specification and distinguish it from libassmod; `.asa` has no provided authoritative spec.

## S4. License and reuse policy

This is a conservative engineering intake policy; file-level provenance and distribution choices must be reviewed before reuse. Original Taiyou license is pending owner selection; proposed MIT OR Apache-2.0 is not granted by writing it here.

| Material | Evidence / obligations to verify | Taiyou policy |
|---|---|---|
| libass | [ISC COPYING](https://github.com/libass/libass/blob/f61db567e6593df3470e91594bcd4ad2d0473aff/COPYING); retain copyright/permission notices for copied material; inspect individual files and third-party components | Study algorithms; any source reuse needs an inventory, notices and review, not automatic translation |
| libassmod | [pinned ISC COPYING](https://github.com/amanosatosi/libassmod/blob/3f5640045bbd737661f3858480ec948fece929cb/COPYING); inspected RGBA file has ISC-style header; this is not an audit of every addition | Same file-level review; extension ideas are not permission to copy all code |
| pinterf xy | [GPLv2 license text](https://github.com/pinterf/xy-VSFilter/blob/25cf3e8433b3a83a5489e47a2c335516492bb815/License.txt); individual headers determine exact grant/version options | Behavioral/source study only. No copied/translated code or linking into a permissive Taiyou release without separate licensing decision |
| Historical Guliverkli2 VSFilter | Local archive now supplied; inspected RTS.cpp header states GPLv2-or-later, with License.txt present; remaining files need individual review | Behavioral study only; no copying/translation into permissive Taiyou without a separate licensing decision |
| libass-tests/fonts/images | Root view alone does not establish all asset licenses | Inventory per asset, retain notices; fetch externally at pin, do not redistribute unknown assets in Taiyou |
| FreeType | [FTL or GPL licensing choice](https://freetype.org/license.html), including credit/notice requirements and bundled components | Prefer reviewed FTL distribution path for proposed permissive project; T002 records obligations |
| HarfBuzz/Rust bindings/Unicode/Rayon/PNG tools | Exact versions and transitive trees not selected | T002 performs license, maintenance and advisory review; no inferred blanket compatibility |

Keep a future `THIRD_PARTY_NOTICES.md` with origin, commit/version, files, license text locations, modifications and chosen distribution obligations. No GPL code pasted into prompts/task implementation as a translation recipe. Behavioral descriptions and independently authored tests are the chosen approach; consult qualified review if a proposed reuse crosses a license boundary. Host-provided fonts and KFX corpora need provenance too.
