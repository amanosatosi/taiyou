# R001 — LegacyXyV1 source-research checkpoint

Status: proposed; **not a frozen, accepted release profile**. Date: 2026-10-19.
Integration base: `4204f3d2a8e9734a6bad81b4716e9b7f74b388fe` (T015 merge).
Proposal revision: `R001-source-checkpoint-1`. Candidate SHA and current Actions
evidence are recorded on the separate, unmerged R001 PR.

## Decision and admission boundary

30 rules: **25 source-established, 2 provisional interpretations, 3 blocked;
0 capture-verified; 0 approved; 0 enabled capabilities**. On the separate approval
axis, 27 decisions are provisional and 3 blocked. “Source-established” establishes
the described code path in the hashed local archive, not universal historical
behavior, capture verification, maintainer acceptance or visual compatibility.

[Rule inventory](../../validation/legacy-rules.json) is a proposal, not a runtime
support registry. No renderer reads it. All `approved`/`enabled` flags are false,
the enabled list is empty, and maintainer acceptance is false. Unknown IDs remain
unsupported. A future admission must bind an exact capture, source/build identity,
fixture, profile revision and human review; this checkpoint cannot auto-promote it.
The checkpoint checker intentionally rejects approval/capture claims: future
promotion needs an explicitly reviewed schema/checker change as well as evidence.

**Safe new LegacyXyV1 rules for dependent implementation: none yet.** Existing
CoreV1 C1–C7/ADR-009/010 behavior remains authorized. T016 can use existing Core
effect contracts, while Legacy border details stay R002-gated. T017 must wait for
captured wrap/collision policy, especially L08. T020 may prepare reviewed finite
tag-policy proposals from O01–O07/T01–T02; T026 needs the captured charset/face
environment for F01–F04/P07. No unsupported rule is assigned for implementation.

## Two bounded evidence sessions

1. Session 1: read task/contracts/compatibility/reference map/sources/testing,
   ADR-011, accepted T015 implementation and reports, then pinterf STS file parsing,
   defaults, style binding and RTS overrides. Verified the existing local
   reference fingerprints before relying on them.
2. Session 2: traced event segments, exact animation boundaries, GDI metrics and
   charset handoff, wrapping, line boxes, allocator eligibility/order/history;
   inspected Guliverkli2 only as separate context and local libass only as a
   comparison source. Recorded additional DSUtil RT2MS hash. Closed source study
   after this session; artifact preparation/CI below is validation, not a third
   investigation. No raster/stroke/blur algorithm investigation or implementation.

## Authority and verified identities

The sole main xy source is the supplied extracted tree
`C:/taiyou-time/refrences/xy-VSFilter-3.2.0.810`. Its README lines 1–6 identifies
pinterf pfmod v3.2.0.810 (20240123), branch xy_sub_filter_rc5, and pinterf's repository.
Its version header is fingerprinted too. There is no Git metadata. **No verified
upstream commit** is assigned to this tree. The earlier lock
`25cf3e8433b3a83a5489e47a2c335516492bb815` remains unchanged and equivalence is
unverified. No archive-container file was supplied/found in the reference tree;
there is no fabricated ZIP hash. This is selected-file identity, not a full-tree
checksum or authenticated build recipe.

[Source manifest](../../validation/proposals/R001/sources.json) records exact
SHA-256 and line counts for all evidence files. Line numbers below are one-based
in these local bytes; they are not links into the different locked commit.

| Source file | SHA-256 |
|---|---|
| pinterf-local: README.md | `daa5075f77078787d9645c6d3d86c726f21c1d19ee6b33eec535979f74af2f89` |
| pinterf-local: License.txt | `19404e184cf45ec991b54e1e6f05f847ad8f13fee2a51f3a2d9a960c8f7cb26b` |
| pinterf-local: src/subtitles/STS.cpp | `59f431e3ac36d99e2b694cdab106a530edaf7bbc8c85410f91647d3ff4578895` |
| pinterf-local: src/subtitles/RTS.cpp | `dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb` |
| pinterf-local: src/dsutil/DSUtil.h | `5f59ad8e34c5244d340e2371c507d4836297c2096e3aca49274923eedc2ec86d` |
| pinterf-local: src/filters/transform/vsfilter/version_in.h | `395251cdc886dc2656355c2ef5feebf1c6939625ce2111b0b526219c25f2247f` |
| guliverkli2-local: License.txt | `19404e184cf45ec991b54e1e6f05f847ad8f13fee2a51f3a2d9a960c8f7cb26b` |
| guliverkli2-local: src/subtitles/STS.cpp | `53cd8bd328a4681482f2640b6bd8bc8b55eb3c009c19637dd2a62c76cc2d8a67` |
| guliverkli2-local: src/subtitles/RTS.cpp | `72ebc5da279aae208df7b025cc39c484e459573fb9bc0a9cfc61d559db708efb` |
| libass-local: RELEASEVERSION | `c023a1fcb3128ceec3c14ea262c66aabd075b94b2424a5c14bef8bc90c69e8f3` |
| libass-local: libass/ass.c | `c27270c7145c5d7ae0a369ed7e2a719cbaa0cf50121f6f5bb9fb8fd3c5172873` |

Local check: `pwsh -NoProfile -File scripts/check-local-references.ps1` passed:
four folders, 24 entry-point fingerprints; missing libass-tests remains explicit.
Additional selected-file hashing used PowerShell Get-FileHash SHA256; reference
trees were read-only throughout. New independently authored ASS/SSA scripts contain
no translated/copied GPL implementation.

The separately supplied Guliverkli2 r114-labelled archive is **not authenticated
as revision r114**. Its STS OpenSubStationAlpha lines 1334–1357 has the same
1280/1024 special case; STSStyle::SetDefault lines 2891–2918 has the same size18,
bold, margins20, shadow3/transparency128 defaults. These are separate historical
source observations only. RTS was fingerprinted for provenance, not used to
adjudicate any pinterf rule. No historical build/capture is claimed.

The local libass tree says RELEASEVERSION 0.17.5 and has no verified commit
equivalence. Its ass.c set_default_style lines 229–247 has size18/bold/margins20/
shadow3 (similar to this xy path); custom_format_line_compatibility lines 811–821
can default missing SBAS to true for custom formats; ass_lazy_track_init lines
1844–1873 contains PlayRes inference. These are **local comparison-source
observations**, not findings from the pinned libass build. The T015 pin/run below
is the actual separate comparison evidence. No local source tree was refetched.

## Current implementation and accepted T015 evidence

Inspected `crates/taiyou-ass/src/{lib,sections,styles,overrides,decode}.rs`,
`crates/taiyou-types/src/track.rs`, render evaluate/layout/renderer, font shape,
and `xtask/src/capture_import.rs`, plus validation/COMPARISON.md. The renderer
exists as the T014 Core vertical slice; Legacy parsing still fails explicitly
(lib.rs lines 75–77), auto wrapping/collision and legacy metrics remain gated,
and strict fill-only rendering rejects border/shadow/decorations. “Core differs”
below is source/contract comparison, **not execution of these proposed fixtures**.

T015 PR #24 merged at this integration base; the PR description's older
8bd2b39 counts were checked against the final accepted head
`d364c1dcd8a4887b961fbe6307572fc61bff41be`, its real job logs and invariant
checks, rather than treating prose as a new run. Human baseline acknowledgment:
[exact final-head comment](https://github.com/amanosatosi/taiyou/pull/24#issuecomment-6253106517).
Acknowledgment permits checking; acceptance/integration comes from the human merge.

| Prior accepted evidence, not an R001 run | Observed result |
|---|---|
| [T015 final comparison](https://github.com/amanosatosi/taiyou/actions/runs/39148444634/job/117737034176) | 101 visual frames: 1 exact, 92 unsupported, 8 INFRA_ERROR; 2 separate parser safety passes; 1 separately disabled source; full command fails honestly |
| [Pinned libass oracle](https://github.com/amanosatosi/taiyou/actions/runs/39148444634/job/117737034171) | 101/101 original corpus frames at both p0 and p2; separate libass evidence, no xy execution |
| [T015 comparator/policy self-tests](https://github.com/amanosatosi/taiyou/actions/runs/39148444634/job/117737001030) | passed; full report accounting and immutable inventory rechecks also passed |
| [T015 corpus gate](https://github.com/amanosatosi/taiyou/actions/runs/39148444632) | passed at accepted final head |

Eight retained infrastructure failures: embedded-font/efont-1000.png,
faux-italics/cff-0000.png, font_nonunicode/shiftjis-1000.png, and five
ssa/alpha frames (0/1000/2000/3000/4000ms); all font ingestion/family gaps.
No infrastructure failure is counted as unsupported or visual success.

Relevant *different* corpus witnesses actually exercised by that pinned run:
scaling_playres (P02), scaled-border-default (P03), ssa/alpha (P07),
font_nonunicode (F02/F03), implicit-pos/bordercollision and shift-dir (L04–L08),
token-parsing/trimming (O06/O07). Their native libass reference reproduction
passes do not establish the result of any newly authored minimal script, nor xy
behavior. R001's 34 new scripts have **no executed libass result** yet. Corpus
fonts/images stay external; no accepted T005/T015 record, threshold, reference
image or support classification changes.

## Rule-by-rule decision table

S = source-established, P = provisional interpretation, B = blocked.
All source citations are pinterf-local unless explicitly stated. Every row links
a proposal script; the fixture manifest gives all variants, exact hashes,
timestamps, viewport, expected observable and unavailable environment.
Expected observables distinguish parameter/state/layout predictions from pixels.
No source-only row supplies a golden image or a numerical pixel tolerance.

| Rule / family | Competing interpretations | Expected local xy path / source evidence | CoreV1 assessment | Evidence / decision / consumers |
|---|---|---|---|---|
| [R001-P01: Absent PlayRes](../../validation/proposals/R001/scripts/P01.ass) / parsing | 384x288 fallback versus viewport-sized coordinates | With both PlayRes fields absent and no .style sidecar, file Open resolves 384x288; at output 320x180, explicit (192,144) maps to (160,90). **Source:** STS.cpp, CSimpleTextSubtitle::Open(CTextFile*), 3291–3302. | same coordinate fallback; Core keeps diagnostics | S / provisional; T020 |
| [R001-P02: One-axis PlayRes](../../validation/proposals/R001/scripts/P02-x1280.ass) / parsing | Reject partial resolution versus derive missing axis | PlayResX=1280 alone derives Y=1024, not 960 or viewport height; Y=1024 alone derives X=1280. Ordinary positive X uses integer X*3/4; positive Y uses Y*4/3 at header encounter. **Source:** STS.cpp, OpenSubStationAlpha, 2042–2064; STS.cpp, CSimpleTextSubtitle::Open, 3291–3302. | differs: sections.rs Header::finish rejects partial pairs | S / provisional; T020 |
| [R001-P03: Missing WrapStyle and SBAS](../../validation/proposals/R001/scripts/P03.ass) / parsing | WrapStyle2/SBAS yes versus WrapStyle0/SBAS no | Fresh subtitle object has WrapStyle=0, collisions=0, SBAS=false. With no headers overriding them, soft \n is a space; outline/shadow scaling is disabled. This is the fresh-object file path, not an unknown host reinitialization path. **Source:** STS.cpp, CSimpleTextSubtitle::CSimpleTextSubtitle, 2442–2456; RTS.cpp, CRenderedTextSubtitle::ParseString, 1966–1971. | differs on wrap default (Core2); same SBAS false | S / provisional; T016, T017 |
| [R001-P04: Fallback style and inherited defaults](../../validation/proposals/R001/scripts/P04.ass) / parsing | Core default size20 normal margins10 versus xy constructor size18 bold margins20 | With no style definitions and host default-style override disabled, missing event style becomes Default then uses m_defaultStyle: Arial18 bold, scale100/100, spacing0, alignment2, margins20, outline2, shadow3; primary white, secondary yellow, outline black have transparency0, shadow black transparency128; charset DEFAULT_CHARSET. Host mutation of m_defaultStyle is excluded. **Source:** STS.cpp, STSStyle::SetDefault, 3614–3645; STS.cpp, CSimpleTextSubtitle::GetStyle, 3032–3049; STS.cpp, CSimpleTextSubtitle::ChangeUnknownStylesToDefault, 2696–2737. | differs: Core Arial20 normal margins10 shadow2 opaque shadow; no capture | S / provisional; T016, T020, T026 |
| [R001-P05: Format field ordering](../../validation/proposals/R001/scripts/P05.ass) / parsing | Dynamic Format mapping versus fixed positional parsing | Swapping Start and End labels only in Format does not swap xy dialogue fields: event remains active at 0.5s. OpenSubStationAlpha has fixed positional style/dialogue readers and no Format dispatch. Core honors swapped labels and obtains a negative interval. **Source:** STS.cpp, OpenSubStationAlpha, 1899–2165. | differs: Core dynamic Format | S / provisional; T020 |
| [R001-P06: Duplicate style definitions](../../validation/proposals/R001/scripts/P06.ass) / parsing | Global last definition wins versus preserve prior event binding | An event read after style D size20 but before differing D size40 remains bound to renamed D2 size20; a later event uses D size40. Identical duplicate definitions are discarded. No universal post-parse last-wins rule. **Source:** STS.cpp, CSimpleTextSubtitle::AddStyle, 2740–2792. | differs: Core resolves all events against final last definition | S / provisional; T020 |
| [R001-P07: SSA v4 style semantics](../../validation/proposals/R001/scripts/P07.ssa) / parsing | ASS alpha/alignment interpretation versus SSA conversion | SSA alignment6 maps to top-center8. AlphaLevel64 applies transparency64 to primary/secondary/outline; shadow transparency128. BackColour supplies outline color, replacing TertiaryColour. ASS instead extracts each color high byte and uses keypad alignment. **Source:** STS.cpp, OpenSubStationAlpha, 1972–2030. | differs: SSA syntax is outside initial Core subset | S / provisional; T016, T020, T026 |
| [R001-O01: Repeated paint tags](../../validation/proposals/R001/scripts/O01.ass) / overrides | First paint wins versus ordered last paint wins | Primary color after \c&H0000FF&\c&H00FF00& is green (RGB 0,255,0); this property uses ordered assignment. Do not extrapolate to placement, alignment or fade. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(evaluate), 2217–2250. | same ordered property rule | S / provisional; T020 |
| [R001-O02: Repeated placement](../../validation/proposals/R001/scripts/O02.ass) / overrides | Last placement wins versus first pos/move wins | For \pos(40,40)\pos(240,140), anchor is (40,40). pos and move share EF_MOVE, so the first valid supported-arity pos/move occupies the slot. org separately takes its first valid two-parameter operation. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(evaluate), 2552–2604. | differs: Core last applicable pos; move/org not rendered | S / provisional; T017, T020 |
| [R001-O03: Repeated alignment](../../validation/proposals/R001/scripts/O03.ass) / overrides | Last alignment wins versus first alignment fixes event | \an7\an3 produces top-left alignment7; initialized negative style alignment lets only the first an/a assignment take effect. Invalid first an falls back to original alignment and still prevents a later an. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(evaluate), 2277–2288; RTS.cpp, CRenderedTextSubtitle::GetSubtitle, 2906–2909. | differs: Core last applicable alignment | S / provisional; T017, T020 |
| [R001-O04: Named/bare reset and original style](../../validation/proposals/R001/scripts/O04.ass) / overrides | Unknown reset leaves state versus resets to event original | Default20, Alt40: \rAlt A uses40; parameterless \fs then B uses original20 (org is not replaced); \rMissing C also uses original20. Bare \r resets style to original; it does not clear separately stored EF_MOVE or alignment. Reset names use exact map lookup. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(evaluate), 2620–2624; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(fs), 2483–2501; RTS.cpp, CRenderedTextSubtitle::GetSubtitle, 2906–2909. | differs: ADR009 unknown r-prefixed token remains unsupported; Core reset carries Style alignment; Core empty fs invalid | S / provisional; T020, T026 |
| [R001-O05: Relative and invalid font size](../../validation/proposals/R001/scripts/O05.ass) / overrides | Signed absolute/additive size versus tenth-of-current relative change | From original20, \fs+2 gives24; following \fs-5 gives12; \fs0 and empty \fs restore original20. These finite small values are distinct from undefined overflow behavior. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(fs), 2483–2501. | differs: Core positive signed absolute size and rejects nonpositive/empty | S / provisional; T020, T026 |
| [R001-O06: Numeric coercion, range and empty parameters](../../validation/proposals/R001/scripts/O06.ass) / overrides | Strict token rejection versus CRT prefix coercion and property-specific fallback | From original normal weight400: \b-1 restores original normal (not bold); \b700 selects700. \fscx-10 clamps to0; \fscx restores original100. \fs25junk consumes numeric prefix25; \fsjunk resolves0 then original20. \1a&H1FF& narrows to byte255 transparency. These are finite small CRT conversions only, not a safe rule for NaN/infinity/overflow. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(evaluate), 2320–2325; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(alpha), 2253–2264; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(scale), 2454–2459. | differs: Core b-1 bold, strict numeric grammar; empty/range errors preserve prior state in recover mode | S / provisional; T020, T026 |
| [R001-O07: Malformed brace blocks](../../validation/proposals/R001/scripts/O07.ass) / overrides | Drop unterminated block versus retain literal bytes | Unclosed text A{\fs40B has no closing brace, so GetSubtitle emits it as text instead of applying fs40. Closed {garbage} is consumed because tag parsing returns true even without recognized operations. No recursive brace grammar is inferred. **Source:** RTS.cpp, CRenderedTextSubtitle::GetSubtitle, 2927–2971; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(tokenize), 2205–2214. | Core recovery/unsupported diagnostics differ; no output comparison | S / provisional; T020 |
| [R001-O08: Nested transform parentheses](../../validation/proposals/R001/scripts/O08.ass) / overrides | Balanced recursive parameter scope versus scan through first right parenthesis | Tokenizer scans to the first ')' rather than balancing nested parentheses; transform's last parameter is recursively tokenized only within that slice. Exact resulting nested-transform text/animation output is not established for this witness; do not enable a general recursive grammar. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(tokenize), 2081–2128; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(tokenize), 2201–2203. | Core t unsupported; no parity claim | P / provisional; T020 |
| [R001-T01: Intervals, zero duration and overlaps](../../validation/proposals/R001/scripts/T01.ass) / timing | Inclusive end/one-frame zero event versus half-open interval | A=[1s,2s), B=[2s,3s), Z=[2s,2s): at 999999us none; 1000000 A; 1999999 A and X; 2000000 B and X; 3000000 none. A second X=[1.5s,2.5s) coexists with A then B; Z has no positive segment. No forced one-frame zero event. **Source:** STS.cpp, CSimpleTextSubtitle::CreateSegments, 3157–3211; STS.cpp, CSimpleTextSubtitle::SearchSubs2, 2929–2970. | same half-open activity; Core diagnostic handling for zero interval differs | S / provisional; T020 |
| [R001-T02: Exact movement/transform boundaries](../../validation/proposals/R001/scripts/T02.ass) / timing | Continuous microseconds versus truncated local milliseconds | m_time is RT2MS(rt-start). For move(40,40,240,40,100,200): t=100ms uses start, t=150ms uses midpoint140,40, t=200ms uses end. At100999us the local clock is100ms; transform fs20->40 over100..200ms yields20/30/40 at100/150/200ms. RT2MS is integer division by10000 reference ticks, so100999us evaluates as100ms for this active nonnegative local time. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseScript, 3155–3158; RTS.cpp, CRenderedTextSubtitle::ParseScript(EF_MOVE), 3175–3200; RTS.cpp, CRenderedTextSubtitle::CalcAnimation, 2856–2870; src/dsutil/DSUtil.h, RT2MS macro, 30–32. | Core move/t unsupported; C3 uses absolute request-time evaluation | S / provisional; T020 |
| [R001-F01: Font size and measured metrics](../../validation/proposals/R001/scripts/F01.ass) / fonts | One-em size versus positive GDI logical height and GDI widths | Rendered fontSize is scaled by script/output Y and64, then CMyFont overwrites lfHeight with positive rounded size; ascent/descent come from GetTextMetrics, width from GetTextExtentPoint32W. Cannot equate font-size20 to a20-unit OpenType em or infer exact advances without the selected GDI face. **Source:** RTS.cpp, CMyFont::CMyFont, 116–136; RTS.cpp, CRenderedTextSubtitle::GetSubtitle, 2955–2961; RTS.cpp, CText::GetTextInfo, 697–722. | differs by design: Core HarfBuzz design-unit em scaling | S / provisional; T017, T026 |
| [R001-F02: Encoding and charset](../../validation/proposals/R001/scripts/F02.ass) / fonts | Encoding ignored for Unicode versus charset-constrained font selection | Negative style Encoding maps to DEFAULT_CHARSET; \fe128 assigns128 and empty \fe restores original charset. LOGFONTW receives charSet even for Unicode text. Byte-script conversion separately uses TranslateCharsetInfo/Windows codepages; Unicode glyph selection and legacy byte decoding are distinct. **Source:** STS.cpp, OpenSubStationAlpha, 2005–2014; RTS.cpp, CRenderedTextSubtitle::ParseSSATag(fe), 2415–2422; STS.cpp, operator<<=(LOGFONTW), 3719–3730; STS.cpp, CharSetToCodePage, 417–422. | Core accepts UTF8 only, encoding1 in evaluated styles; no fe support | S / provisional; T026 |
| [R001-F03: Missing family and fallback glyph identity](../../validation/proposals/R001/scripts/F03.ass) / fonts | Deterministic catalog fallback versus Windows mapper/font linking | CMyFont retries Arial only if CreateFontIndirect fails; successful font creation does not prove the requested face exists. Exact face/glyph for a deliberately missing family, Arabic, CJK and private-use U+E000 remains unknown until resolved face/font files and OS linking are captured. **Source:** RTS.cpp, CMyFont::CMyFont, 116–136. | different mechanism: explicit catalog; exact pixel/glyph result unmeasured | B / blocked; T026 |
| [R001-F04: Paint-boundary shaping and spacing](../../validation/proposals/R001/scripts/F04.ass) / fonts | Shape across paint changes versus separate GDI chunks | ParseString constructs CText separately around overrides; measurement precedes line compaction. CWord::Append refuses differing complete styles. Nonzero spacing measures/draws one UTF16 code unit at a time. fi and Arabic joining across a color boundary may differ; exact glyph/advance changes are provisional, not a claim that GDI never shapes. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseString, 1966–1984; RTS.cpp, CWord::Append, 181–194; RTS.cpp, CText::GetTextInfo, 697–722; RTS.cpp, CText::CreatePath, 646–687. | Core preserves shaping across paint spans and first-logical-scalar cluster paint per ADR010 | P / provisional; T017, T026 |
| [R001-L01: Hard/soft/nonbreaking escapes](../../validation/proposals/R001/scripts/L01.ass) / layout | Soft n always hard versus WrapStyle-dependent break | For q2, A\nB\NC\hD has three lines: A / B / C NBSP D. For q0/1/3 soft n becomes ordinary space; N always hard; h becomes NBSP, not a word delimiter. Style metrics govern empty breaks; GetNextLine halves ascent/descent for an empty line. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseString, 1966–2020; RTS.cpp, CSubtitle::GetNextLine, 1463–1472. | same q2 escape behavior; empty-line metrics differ from Core full style metrics | S / provisional; T017 |
| [R001-L02: WrapStyle1 automatic lines](../../validation/proposals/R001/scripts/L02.ass) / layout | Break every glyph versus greedy whitespace groups | Mode1 leaves available width unchanged. GetNextLine groups consecutive whitespace/nonwhitespace CWords, accepts a group if it fits or line is empty, otherwise retries it on next line; overlong first word is retained. Exact break words require captured font metrics, not guessed glyph widths. **Source:** RTS.cpp, CSubtitle::GetNextLine, 1453–1503; RTS.cpp, CSubtitle::GetWrapWidth, 1442–1448. | Core automatic wrapping unsupported | S / provisional; T017 |
| [R001-L03: WrapStyle0 and3 balancing](../../validation/proposals/R001/scripts/L03-q0.ass) / layout | Generic aesthetically balanced lines versus archive-specific width heuristic | For full line width F and available M>0, minwidth=F/(abs(F)/M+1) with integer division. Accumulate words while width<minwidth and abs(width+word)<abs(M). Mode3 additionally subtracts last word when width<F and F-width+lastWord<M; mode0 does not. Then GetNextLine uses that width. Concrete break words cannot be fixed without GDI measurements. **Source:** RTS.cpp, CSubtitle::GetWrapWidth, 1421–1450. | Core modes0/3 unsupported | S / provisional; T017 |
| [R001-L04: Alignment and implicit margins](../../validation/proposals/R001/scripts/L04.ass) / layout | Viewport midpoint versus margin-box center; zero event margin inherits | Zero event margins inherit original event-style margins before scaling. Without pos/move, MakeLines chooses left=marginL, center=(marginL+width-marginR-boxWidth+1)/2, right=width-marginR-boxWidth; bottom=height-marginBottom-boxHeight, middle=(marginTop+height-marginBottom-boxHeight+1)/2, top=marginTop (integer internal units). Resetting to Alt does not replace original event margins. **Source:** RTS.cpp, CSubtitle::MakeLines, 1537–1561; RTS.cpp, CRenderedTextSubtitle::GetSubtitle, 2978–2988. | Same horizontal margin-box anchor principle; GDI box metrics/rounding differ. Core middle Y is viewport midpoint; no pixel parity established. | S / provisional; T017 |
| [R001-L05: Collision eligibility and direction](../../validation/proposals/R001/scripts/L05.ass) / layout | All events collide/reverse swaps direction versus gated same-layer allocator | Fresh allocator displaces only same-layer intersecting rectangles. Bottom align1..3 search up; middle/top4..9 search down. pos/move, org and animated style transforms bypass allocation. collisions=reverse is parsed but allocator explicitly has TODO and does not implement reversed collisions. **Source:** RTS.cpp, CScreenLayoutAllocator::AllocRect, 1606–1651; RTS.cpp, CRenderedTextSubtitle::ParseScript, 3258–3259. | Core renderer currently omits collision adjustment; C4 excludes pos/move but requires stateless active-set placement | S / provisional; T017 |
| [R001-L06: Border-influenced layout bounds](../../validation/proposals/R001/scripts/L06.ass) / layout | Ink-only boxes versus line border maxima in placement | Line stores rounded max outlineWidthX/Y; spaceNeeded width=max(lineWidth+borderX), height=sum(ascent+descent). First/last line borderY affect allocator rectangles via CRect(0,topBorder,0,bottomBorder). This describes layout inputs only; no stroke/blur algorithm or exact ink expansion is approved. **Source:** RTS.cpp, CSubtitle::MakeLines, 1537–1561; RTS.cpp, CSubtitle::GetNextLine, 1463–1470; RTS.cpp, CScreenLayoutAllocator::AllocRect, 1613–1619. | Core fill-only; T016/T017 must not infer stroke from these bounds | S / provisional; T016, T017 |
| [R001-L07: Multi-event order](../../validation/proposals/R001/scripts/L07.ass) / layout | Sort by start time versus layer then source readorder | ParseScript sorts active events by ascending layer then readorder; file ingestion defaults readorder to entry ordinal. Overlapping explicit-position events with same start/layer remain source-ordered. Equal custom readorder tie order is not established and excluded. **Source:** RTS.cpp, lscomp / CRenderedTextSubtitle::ParseScript, 3103–3154; STS.cpp, CSimpleTextSubtitle::AddSTSEntryOnly, 2664–2685. | same layer/source ordering for file events; no parity claim for custom readorder | S / provisional; T017, T020 |
| [R001-L08: Seek-history collision policy conflict](../../validation/proposals/R001/scripts/L08.ass) / layout | Keep prior allocated B position versus recompute from current active set | Allocator retains active entries across adjacent segments. With A0..1s and B0.5..2s, sequential sampling at0.75s then1.25s can preserve displaced B after A ends; fresh seek to1.25s can place B at its default anchor. This is a source-derived conflict candidate, not a captured outcome. C1 forbids history dependence: keep this capability blocked; architect must choose a deterministic canonical policy or explicit separately reviewed contract change. **Source:** RTS.cpp, CScreenLayoutAllocator::AdvanceToSegment, 1581–1603; RTS.cpp, CScreenLayoutAllocator::AllocRect, 1606–1618; RTS.cpp, CRenderedTextSubtitle::ParseScript, 3137–3149. | conflicts with C1 history-independent seeking and C4 recompute-active-set rule | B / blocked; T017 |
| [R001-O09: Extreme malformed numeric domain](../../validation/proposals/R001/scripts/O09.ass) / overrides | Clamp/safely reject versus CRT-dependent overflow/NaN propagation | No exact legacy rule established for 1e999, NaN or integers outside CRT range. wcstod/wcstol plus unchecked casts do not establish safe portable semantics. Probe captures must run bounded in disposable Actions Windows environment; Taiyou limits/error guarantees remain mandatory. **Source:** RTS.cpp, CRenderedTextSubtitle::ParseSSATag(fs), 2483–2501. | Core rejects nonfinite/exponent/overflow tokens; retain that safety contract | B / blocked; T020, T026 |

Missing ScriptType is not silently assumed to be ASS: OpenSubStationAlpha
initializes version/sver=3 at STS 1903; ScriptType updates both at 2081–2085,
while [V4+ Styles] updates only sver at 2104–2107. Dialogue parsing still tests
version at 1932–1933. That directly observed parser distinction does not establish
a host-independent result for all missing-header files; the current profile
does not admit that malformed/missing-type family. Similarly, truncated Style
columns do not inherit arbitrary omitted fields: fixed readers can fail and
Open can reject the file. R001-P04 is the absent-style fallback, not permission
to fill malformed style records from constructor defaults.

Unusual duplicate PlayRes headers, invalid/nonpositive resolutions, custom
readorder ties, HTML tags/effects, vertical fonts and non-square aspect remain
outside the approved set (which is empty). R004 owns detailed resolution/aspect
transforms. R002 owns stroke, raster and blur arithmetic; L06 only identifies
which layout fields consume outline widths.

## Concrete conflict requiring architect decision

R001-L08 has a minimal two-event witness in scripts/L08.ass. Compare a fresh
renderer seeking directly to 1.25s against sequential requests at 0.75s then
1.25s. AdvanceToSegment retains B's allocated rectangle when the segment distance
is at most one; AllocRect returns that stored rectangle. Source order A then B
lets A displace B before A ends. C1 requires history-independent seeking and C4
requires recomputing from the active set.

Choices for review: (a) define a deterministic, explicitly bounded Legacy policy
and document its captured deviation from sequential xy history; or (b) propose a
separate ADR changing the relevant contract before dependent implementation.
No choice is made here, no contract is changed, and no observed historical pixel
result is invented. Independent source findings are preserved.

## Fixture and capture environment

[Fixture manifest](../../validation/proposals/R001/fixtures.json) contains 34
independently authored, UTF-8 BOM minimal scripts (LF, raw-byte SHA256), including
four one-axis PlayRes variants and separate q0/q3 scripts. They are proposals,
not accepted corpus tests. q is event-global: q0/q3 are separate scripts so a
later q does not accidentally change the earlier line's wrapping. For other
rules, each script isolates the competing interpretation described in its row.

Declared requested family Arial is **not a font file identity**. Expected font
metrics, line-break word indices, resolved glyph IDs and pixels stay pending.
For font-sensitive variants use the same measured face files in both runs,
record fallback selection; do not silently supply a different Arial clone.
SSA P07 uses explicit v4 fields; legacy-byte/codepage scripts need separately
preserved original bytes and decoding provenance before T026 admission.

No executable/DLL was found by a bounded `rg --files` search for exe/dll/zip/7z
under the supplied reference root. No authenticated xy binary, controlled runner
image/font inventory, locale/codepage, video pipeline or original captured output
was supplied. This does not assert that no executable exists anywhere on the PC.
The PC was not used to execute native code. **Captures obtained: zero.**

All actual environment fields and output/hash fields are null in the manifest,
rather than filled with the editing PC's settings. For a real capture record:

- Binary file/build/version/architecture, SHA256, full source identity or verified
  binary-to-source relation and all host/filter configuration.
- Windows version/build/architecture; locale, ANSI/OEM codepages; actual selected
  font files, face indices, SHA256 and fallback/linking identities.
- Original input bytes/hash, exact timestamps and seek sequence; frame dimensions,
  frame/sample aspect, video input/output pixel formats and color matrix/range.
- Host/render/output pipeline, raw original capture path/hash and reproducible
  procedure; no desktop screenshots with unknown scaling/color conversion.
- For alpha reconstruction, controlled black/white raw captures, verified numeric
  reconstruction and originals retained; reconstruction is currently **unvalidated**.

T015 `capture-import` is an unconditional INFRA_ERROR/failure placeholder, not
a certified importer or schema for actual captures. No input exists to import
and no import/comparison is falsely reported as executed. The new research
checker validates proposal provenance and rejects fake capture/approval evidence;
it does not replace T015's trusted pixel comparator or certify captured pixels.

## Minimum follow-up and acceptance gates

| Follow-up | Smallest concrete action / owner decision |
|---|---|
| R001-E01 source/build identity | Supply one approved pinterf xy binary (Windows Actions/external controlled runner), SHA256 and full matching source/build relation; verify archive-to-pin identity or review the distinct archive before treating it as oracle |
| R001-E02 controlled captures/import | Supply recorded OS/font/locale/pipeline environment; implement/review the currently missing T015 capture-import validation separately; capture high-impact P01–P07/O01–O07/T01–T02 first; preserve original bytes and output hashes |
| R001-E03 metrics/wrap comparison | Capture F01–F04/L01–L07 with measured GDI metrics and pinned font files; run these exact proposals through pinned libass separately; report glyph/line boxes and numerical output differences without baseline promotion |
| R001-E04 conflict/unsafe domain | Architect adjudicates L08 using fresh/sequential captures. O08 nested scope and O09 extreme numeric outputs remain gated; bounded native probes run only in Actions, and cannot waive Taiyou's malformed-input safety guarantees |

R001 is **only a source-research checkpoint**, not fully accepted. Missing binary,
unverified archive/commit equivalence, absent captures/alpha validation, absent
new-fixture libass results and L08 conflict leave the Legacy release gate closed.
A maintainer may accept this checkpoint without accepting any behavior capability.

## Validation and review procedure

Locally permitted checks: source fingerprints, selected-file/script SHA256,
Markdown structure, JSON/accounting and Git diff checks. All executable test
suites, Cargo/native/rendering work run on Actions only. The additive
`R001 research evidence` job checks documentation/inventory/fixture consistency
and rejects synthetic invalid evidence; existing Actions run the T015 comparator
self-tests, available accepted focused Core/libass comparison and applicable fast
suite. No performance measurement or improvement is claimed.

These additions live under protected validation/** and add a workflow. The exact
PR-base Corpus policy gate must print the real head/base/change-digest human
acknowledgment before it can pass; **an agent must not post that acknowledgment**.
See [ADR-011](../decisions/011-solo-maintainer-validation.md) and
[maintainer process](../../validation/MAINTAINER-SETUP.md).
No accepted tool/policy is modified, no guard bypass is introduced, and the PR
stops for technical/maintainer review without merging. Current run/job URLs,
pass/fail/blocked results and candidate SHA belong in the PR completion report;
old T015 evidence above is explicitly not a new-candidate pass.
