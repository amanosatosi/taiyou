# T017 — Core policy proposals for owner decision

Status: F1 was subsequently **owner-approved for CoreV1**; see
[ADR-012](../decisions/012-core-collision-footprint.md). W1 below was rejected
pending revision; [W2](T017-wrapping-policy-v2.md) supersedes it as a proposal.
Neither W1 nor W2 is implemented or approved. Applies only to CoreV1.
Reviewed implementation: `9eee96fe16989089d8fd3f65b1bb136455ffca7e`.
The two decisions below become separate ADRs and explicit C4 amendments only
after human owner approval and architecture review. They do not imply historical pixel identity. ADR-011 baseline acknowledgment is a separate final step
for the exact protected-change candidate, not a substitute for this decision.

## Proposal F1 — Core geometric effect footprint

Keep the existing local logical/ink union as the base reservation (including
whitespace and full empty-line metrics). For every positioned glyph with known
nonempty ink, transform the four corners of its font ink rectangle through the
same size, run scale, baseline, rotation/origin and script-to-device affine map
as C5. Take the device-space axis-aligned hull. Unknown ink is an error, never a
font-size estimate. This is a conservative geometric hull, not a bitmap hull.

For each compatible C5 group, union these glyph hulls to obtain I. Let r be the
actual Core isotropic border radius in device pixels. It is border size when
SBAS=false, or border size times the common device/script ratio when SBAS=true.
Use the same nearest/ties-away Q6 radius as the stroker, divided by 64; r=0 has
no expansion. Let E be I expanded by r on all sides (round widening's AABB).
Union into the base reservation:

- I when primary opacity is nonzero;
- E when a positive Outline border and outline opacity are nonzero;
- E translated by the actual Q6 shadow offset when a border exists and shadow
  opacity is nonzero, otherwise I translated by that offset for a visible shadow.

Transparent outline paint does not suppress E as the source of a visible shadow.
Transparent shadow paint adds nothing and performs no offset quantization.
Zero-offset visible shadow still contributes its source support. An empty ink
hull contributes no effect support. Taking E rather than the E-F ring's holes is
an intentional conservative rectangle policy. Invisible effects add no padding;
the base logical reservation remains even for entirely transparent text.

Convert device hulls back to script coordinates before union with the base box.
Shadow offsets scale independently on x/y only when SBAS=true. With SBAS=false,
device widths/offsets convert back by the respective mapping ratios. SBAS=true
isotropic borders with unequal x/y ratios remain unsupported exactly as C5;
this proposal does not admit anisotropic stroke. Unequal mapping with border0,
or SBAS=false borders, is defined. Rotation precedes device-space expansion and
shadow translation; border width does not rotate/scale with glyph contours.
Apply per-run effects before the event union. Negative/offscreen bounds are kept.

Coordinates stay checked finite f64. Only the already specified effect Q6
parameter quantization is used; do not floor/ceil boxes to bitmap pixels, add an
epsilon, or invent antialias padding. The footprint moves continuously with the
event anchor and C4 chooses minimum displacement in script space. This policy
separates geometric support: antialias coverage at touching geometric edges may
share a boundary pixel. It does not promise zero shared raster pixels or xy
pixel parity. If strict pixel-support separation is desired, that is a different
policy requiring a phase-aware rounding/placement decision before implementation.

Exact synthetic fixtures (unit mapping, top alignment, natural anchor 0, base
and I=[0,0,10,10], all mentioned paints visible unless stated):

| Input | Expected local footprint / second anchor Y |
|---|---|
| Two events, bord2 each | [-2,-2,12,12] each; second Y=14 |
| Two events, bord5 each | [-5,-5,15,15] each; second Y=20 |
| First bord5, second bord2 | second Y=17 |
| bord2; shad=(3.5,-2.25) | [-2,-4.25,15.5,12] |
| Same with shadow alpha0 | [-2,-2,12,12] |
| bord2, outline alpha0, shadow alpha0 | base [0,0,10,10] |
| bord2, outline alpha0, visible shadow=(3.5,-2.25) | union base with [1.5,-4.25,15.5,9.75] |
| SBAS=false, sx=2,sy=3, bord2 | script padding=(1,2/3) |
| SBAS=true, sx=sy=2, bord2 | script padding=(2,2) |
| SBAS=true, sx=2,sy=3, bord2 | existing UnsupportedFeature |
| SBAS=false shadow1, sx=2,sy=3 | script shadow=(1/2,1/3) |
| SBAS=true shadow1, sx=2,sy=3 | script shadow=(1,1) |

Also verify 90-degree rotated I=[0,0,10,4] maps to [0,-10,4,0]
before device border expansion; multiple differing run effects union separately;
fractional radii use actual Q6 values; zero-size/space-only ink adds no padding.
Pinned-font ASS integration compares displacement changes with borders, hidden
shadows, rotated styles, explicit positions, scaling and direct/sequential seeks.
No existing expected asset is changed to establish these fixtures.

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

## Approval and implementation sequence

Owner may approve F1 and W1 independently, or give exact changes. Until approval,
only explicit filtering, interval-search optimization and bounded staging work
may change production behavior under existing C4. After approval: publish the
separate ADR and C4/C7 amendment; implement numeric footprint/wrap oracles; run
fixed-font stage and RGBA tests; run all existing Actions gates/full comparator;
request final exact-SHA protected-change acknowledgment under ADR-011. Neither
approval nor acknowledgment authorizes changing frozen tests or references.
