# T017 — Historical Core policy proposals

Historical review context, not an active approval gate. W1 was rejected and
subsequently superseded by the explicit Unicode opt-in proposal in
[ADR-013](../decisions/013-core-unicode-wrapping.md). Ordinary verified ASS/SSA
corrections follow [compatibility findings](../compatibility-notes.md).

## Retired F1 collision proposal

F1 introduced geometric effect bounds, but its invisible-border exclusion was
incorrect. Pinterf reserves transparent first/top and last/bottom line borders.
The obsolete behavioral specification and example expectations are retired;
[current source evidence and measurements](../compatibility-notes.md#layout-positioning-and-collision)
replace them. Engineering safety and protected validation remain intact.

## Proposal W1 — Deterministic Core automatic wrapping

Historical pinterf source defines standard escapes/grouping/wrapping separately. The following is an explicit `\\warpmode1` Core extension policy, **not xy emulation**.

1. Available width is PlayResX minus inherited left/right margins. Nonpositive
   width returns InvalidArgument for automatic modes. Explicit positioning does
   not change this width. WrapStyle2 is unchanged and does not wrap.
2. Hard N terminates a paragraph in every mode, preserving blank paragraphs and
   full style metrics. Soft n is hard only in mode2; in modes0/1/3 it is U+0020.
   No-break h is U+00A0, never a break opportunity.
3. Initial Core automatic opportunities are maximal U+0020 space groups between
   nonspace text groups. Other Unicode whitespace, tabs, hyphens, ideographs and
   U+00A0 are not implicit opportunities. This deliberate limited repertoire
   must be documented; adding UAX14 or other boundaries is a separate policy.
   Never split a grapheme/cluster; a space with attached combining content is
   retained as text and cannot create an interior-cluster break.
4. At an inserted break consume the entire delimiter space group: it remains in
   source provenance but contributes no glyph/advance/paint on either line.
   Preserve spaces inside a line and paragraph-leading/trailing spaces. A
   space-only paragraph remains one full-metric line, possibly wider than width.
5. Each candidate line is shaped with its actual new boundaries and retained
   original decoded spans/style associations, full font fallback, language and
   per-line bidi rules. No cached paragraph glyph slicing or synthetic original
   spans. Paint boundaries do not force shape boundaries. Fit uses T010 logical
   advance extent including fractional scale/spacing, with exact <= comparison.
   No pixel rounding or epsilon. A single unbreakable group may exceed width;
   do not split it or silently shrink text.
6. Mode1: choose the furthest logical break whose freshly shaped line fits.
   Examine all legal ends because advances need not be monotonic. If no end
   fits, emit the first unbreakable group. Repeat from the next group.
7. Modes0/3: enumerate admissible plans using the same candidate line widths.
   Minimize line count, then sum of squared slack max(0,width-line_width)^2.
   A line exceeding width is admissible only for one unbreakable group (or the
   preserved space-only paragraph). With equal cost, mode0 prefers the
   lexicographically larger vector of line widths; mode3 prefers smaller.
   Final tie: lexicographically earliest decoded break-offset vector. Calculate
   costs in fixed logical order, checked finite f64, without epsilon or FMA.
8. Publish only chosen line shapes, visual run order, source maps and normal
   T010 max ascent/descent baselines. Form logical/ink/collision boxes after
   wrapping; then collision placement runs over the complete active set.

Bounded work proposal: add caller-configured `wrap_candidates_per_event`, default
10,000, and `wrap_shaped_glyphs_per_event`, default 1,000,000, to Limits/C7.
Count every candidate shaped, including repeated candidates, and actual total
glyphs produced during search; exceeding either yields LimitExceeded without
truncating the break plan. Transient memory remains under existing Limits.
Memoize only within this single layout call, with fallible reservation and
owned charge release; no T019 retained cache. A quadratic candidate table is
permitted only if it fits both work and memory budgets. Empty paragraphs count
as one candidate. Limits and counters enter reports; no silent greedy fallback.

Executable synthetic expectations, monospace units, full ascent8/descent2,
spaces width1, letters width1, available width6 unless stated:

| Input | Mode | Expected lines / metrics |
|---|---|---|
| aa bb cc | 1 | aa bb / cc; widths5,2; heights10,10 |
| aa bb cc | 0 | aa bb / cc (equal squared-slack tie, longer first) |
| aa bb cc | 3 | aa / bb cc (same cost, shorter first) |
| aa + three spaces + bb cc | 1 | aa   bb cannot fit; aa / bb cc |
| aaaa h bbbb | 0/1/3 | one unbreakable 9-unit group, allowed overflow |
| aa soft-n bb hard-N cc | 0/1/3 | aa bb / cc |
| aa soft-n bb hard-N cc | 2 | aa / bb / cc, existing behavior |
| aa hard-N hard-N bb | all | aa / empty / bb; total height30 |
| abcdefg bb | 0/1/3 | abcdefg / bb; no word split |
| aa bb; width5 exactly | 0/1/3 | one width5 line |

Additional acceptance fixtures must pin real font IDs and test inherited
margins, fractional advances/spacing, style/font changes inside words, NBSP,
RTL mixed with Latin/numbers, combining marks, ligatures, Arabic contextual
forms changing at new line boundaries, and exact source-interval ownership.
Independent candidate shaping must verify each selected line, not merely its
parser state. Test ties with unequal font metrics and zero-width glyphs; compare
repeated requests and worker-independent plans. The synthetic expectations above
are proposed arithmetic oracles, not invented font widths or golden images.

Historical pixel-parity claims may additionally use authenticated pinterf outputs for
L01–L03 on the original proposals, pinned fonts/GDI metrics, narrow widths that
distinguish modes0/3, fractional spacing, long words, whitespace, bidi and
context-sensitive boundaries. Record binary/font/Windows/locale/pipeline/input/
timestamp/output hashes. Source or libass results cannot substitute for captures.

## Historical review context

This snapshot does not impose an ADR/research admission gate on ordinary
source-backed compatibility corrections. The explicit Unicode extension retains
its data licensing, resource and public interface safeguards. Protected fixtures,
full CI and human merge/baseline ownership remain unchanged.
