# Reduced visual witnesses

Taiyou-owned reductions for manual inspection, introduced by the maintainer's
visual compatibility review request. No expected images or authenticated xy
captures exist. These scripts are supplemental and cannot accept/promote any
corpus result. No external assets or frozen fixtures are changed.

Run `python scripts/render-review-witnesses.py --review PATH --corpus PATH` in
GitHub Actions after the existing renderer build. It uses the pinned corpus's
DejaVu Sans font, strict execution and the existing bounded subprocess adapter.
Scripts and candidate PNGs go directly into the behavior folders, with one
shared `WITNESSES.txt`; corpus frame counts exclude them.

| Script | Isolated behavior/control |
|---|---|
| kt.ass | Explicit second sweep start vs sequential karaoke |
| run-splits.ass | Italic split inside one word vs unsplit word |
| collision.ass | Two automatically placed events, no effects |
| layoutres.ass | Anisotropic LayoutRes/PlayRes with one word |
| scaled-border.ass | Same word rendered with SBAS yes/no |
| alpha.ass | Animated 0x1FF vs 0xFF alpha target, no geometric clip |
| faux-italic.ass | Synthetic italic vs upright on one face |
| drawing.ass | One closed rectangle, no font geometry |
| trimming.ass | Example vs ***Example with trailing style-name spaces |

The pinterf local archive's `src/subtitles/RTS.cpp` is source evidence, not an
image oracle: `CMD_kt` sets start/end to the supplied centisecond time (2512–2520),
`CMD_kf/K/k/ko` advance the current timing (2522–2551), and alpha conversion casts
after `CalcAnimation` (2260–2274). Run splits create `CText` words carrying the
current timing (1979–2007). The selected-file reference check passed locally;
archive equivalence to the locked upstream commit remains unverified. See
[local references](../../../docs/LOCAL_REFERENCES.md).
