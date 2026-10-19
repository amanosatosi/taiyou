# T017 W2 — Unicode-aware Core wrapping proposal

Status: **Core semantics owner-approved in review; final contract confirmation
and implementation admission pending**. Supersedes W1's space-only proposal.
The owner approved the Unicode15.1 data plan in principle and the backwards DP
approach subject to the rounding clarification proposed below and in
[draft ADR-013](../decisions/013-core-unicode-wrapping.md). Production limits,
generated data and automatic wrapping remain unapproved/unimplemented.
F1 is separately approved in
[ADR-012](../decisions/012-core-collision-footprint.md). This proposal does not
admit R001 L01–L03 or claim historical xy-VSFilter behavior.

The advances-only API and search tests in this PR are **experiments**, not an
automatic wrapping implementation. Their full-line equivalence and workload
measurements do not admit missing data/conformance/native-work prerequisites.

## Exact opportunities and atoms

Pin Unicode **15.1.0**, [UAX14 revision51](https://www.unicode.org/reports/tr14/tr14-51.html)
default line-breaking rules and [UAX29 revision43](https://www.unicode.org/reports/tr29/tr29-43.html)
extended grapheme boundaries. This deliberately stable version is a policy pin,
not a claim to use the latest Unicode release. A future implementation must
identify/hash its data tables and pass the matching LineBreakTest and
GraphemeBreakTest before conformance is claimed. No new library is selected by
this proposal; version/license/data-size/alternatives review precedes admission.

Apply UAX14 to decoded text across style/paint boundaries, then intersect allowed
opportunities with extended grapheme boundaries and paragraph shaping-cluster
boundaries. A boundary inside any participating fallback/bidi run's cluster is
removed. Every resulting candidate line is freshly shaped with its real context
and source ranges. No grapheme, emoji ZWJ sequence, combining sequence or existing
shaping cluster is split. Per-line bidi resolution/reordering follows C4; visual
order never determines logical break selection. Font changes cannot introduce a
break forbidden by the text boundary policy.

Use default LB1 resolution (including CJ→NS and SA's combining-mark treatment).
No dictionary segmentation, language-specific hyphenation or locale-dependent
tailoring. Japanese/CJK/precomposed Hangul use the default Unicode opportunities,
including opening/closing punctuation restrictions. Thai/Lao/Khmer sequences
that need a dictionary remain atomic unless an explicit legal separator exists;
do not invent character-by-character emergency breaks. Unknown Unicode 15.1
characters use that version's default properties, not the host Unicode database.

Proposed data choice: independently implement the default boundary rules over
vendored, hashed Unicode15.1 property tables generated from LineBreak.txt,
GraphemeBreakProperty.txt, emoji-data.txt, UnicodeData.txt, EastAsianWidth.txt and
DerivedCoreProperties.txt (including Indic_Conjunct_Break). General categories
are needed by LB1 and quote handling; East Asian Width is needed by LB30 in
[the pinned annex](https://www.unicode.org/reports/tr14/tr14-51.html).
Vendor the matching conformance inputs and [Unicode license](https://www.unicode.org/license.txt)
with provenance in a separately reviewed additive proposal. Do not use the host
Unicode database. This adds no Rust/runtime dependency. Existing unicode-bidi
0.3.18 and unicode-script0.5.8 remain the shaping/itemization dependencies; they
do not by themselves provide the proposed pinned line/grapheme algorithm.
Alternatives are a pinned line-break plus segmentation crate with matching data,
or ICU4X segmentation with matching data. Existing ICU case-map tables are not
proof of matching segmentation data. Table hashes, generated size, conformance
results and license review remain prerequisites; no tables or dependency are
admitted by this document.

Core-specific tailoring:

1. ASS hard N always terminates a paragraph. Modes0/1/3 convert soft n to U+0020;
   mode2 retains all existing behavior. Literal Unicode mandatory breaks in
   automatic modes follow UAX14 BK/CR/LF/NL, with CRLF one boundary. Preserve
   explicit empty paragraphs and their full style metrics. Do not change mode2.
2. h is U+00A0. Suppress breaks immediately before/after NBSP and WJ; no-break
   content stays with its neighbors. Default ZWJ and grapheme restrictions apply.
3. A selected break after an interior maximal U+0020 delimiter group consumes
   that entire group (no glyph/advance on either line), retaining original source
   intervals as consumed provenance. Other Unicode separators follow UAX14;
   their glyph/advance remains on the preceding line unless mandatory or ZWSP.
   ZWSP consumes no advance/paint but retains its source interval. Never apply
   generic Unicode trim to a source range.
4. Suppress opportunities that would detach paragraph-leading spaces from the
   first text atom, or paragraph-trailing spaces from the last. Edge spaces
   remain verbatim and contribute advances. An all-space paragraph is one atom.
   No automatically introduced empty lines. Interior delimiter groups never
   become independent overwide atoms.
5. Literal TAB is UnsupportedFeature pending a tab-stop metrics contract;
   discretionary SHY is UnsupportedFeature pending glyph-insertion/provenance
   policy. Do not silently render them as ordinary spaces or ignore them.

An **atom** is the smallest remaining nonempty decoded segment between legal
opportunities after this tailoring, including inseparable leading/trailing
spaces. This definition covers unbreakable words, no-space scripts with no legal
boundaries and font-cluster-suppressed opportunities without guessing glyph
widths. It also gives an admissible plan for every overwide edge-space sequence.

## Fit, admissibility and deterministic selection

Available width is PlayResX minus inherited left/right margins. Nonpositive
available width in automatic modes returns InvalidArgument, including for
explicitly positioned events. Candidate ranges retain original source maps,
per-run style/font/language/fallback and logical direction. Re-shape at each new
line boundary; reset line bidi context and calculate T010 logical advance extent,
including fractional spacing and negative/zero advances. Paint does not split
shape runs. Use checked finite f64 and exact <= fit, without pixel rounding,
epsilon or FMA. Never silently shrink or split an atom.

A line is admissible if its freshly shaped width fits, **or if it contains exactly
one atom**, even when that atom plus its attached edge spaces exceeds width.
An all-space paragraph is one full-metric line with its actual width, even if
overwide. Every selected line must be admissible; no hidden greedy fallback.

- Mode1 selects the furthest fitting logical end among all ends at the current
  start. If none fits, emit the first atom alone. Continue after its consumed
  delimiter. Do not assume candidate widths are monotonic.
- Modes0/3 use the proposed recursive DP comparator defined below: minimize
  line count, then rounded squared-slack score over selected suffix plans.
  This is not exact global minimization over all complete plans. Equal-score
  candidates at a state use mode0's
  lexicographically larger line-width vector; mode3 chooses smaller. Final tie
  is lexicographically earliest decoded break-offset vector. Evaluate costs in
  fixed logical order, with no epsilon or unspecified hash order.

Publish only selected line shapes, their visual run order and full provenance.
Use T010 max ascent/descent including empty-line style metrics. Build distinct
logical/ink/local boxes after wrapping, then ADR-012 effects footprint, then
complete-active-set collision placement. No frame-history or retained wrap cache.

### Candidate reuse and polynomial search (proposed refinement)

Evaluate the ASS event once. Retain the original decoded map and immutable style
ranges; a candidate creates a line-boundary view with original source intervals,
not a newly generated ASS document. Re-itemize bidi for that actual line and
shape its actual context. Use the authenticated immutable catalog, not repeated
font authentication. The advances experiment skips ink extents but preserves
actual fallback, glyph IDs/positions, face metrics, clusters and provenance.
Reject scale seams inside a cluster consistently with full layout. Do not infer
line widths from paragraph glyph slices or additive character measurements.

Key a candidate cost by its exact logical start/end and consumed-delimiter rule.
Measure every unique range at most once during this event's search; checked
triangular storage is O(B squared), where B is the boundary count. No candidate
width monotonicity assumption is allowed. Mode1 scans all remaining legal ends
and selects the furthest fitting one; overwide-single-atom admission is unchanged.

For modes0/3, use a DAG of admissible candidate edges and dynamic programming,
not enumeration of all complete break plans. At each start, choose the best
edge plus already selected suffix by line count, cost, width-vector tie and
break-offset tie in that order. Specify floating evaluation as the backwards
recurrence `cost(start) = squared_slack(edge) + cost(end)`; calculate each slack
and square with separate finite operations, no FMA. This explicit association
is the clarification submitted for final owner confirmation. The independent
experiment compares this recurrence against exhaustive small plans with exact
small dyadic costs and nonmonotonic widths; that agreement is not a claim about
global optimization of arbitrary rounded floating-point costs.
Its simple tie-vector implementation has O(B cubed) worst-case scalar/copy work
and O(B squared) storage, but only O(B squared) boundary shapes; no exponential
search or repeated native shaping of equal ranges. A future more compact tie
ranking requires equivalent fixtures before replacing that ordering.

The selected lines still receive full independent boundary shaping with ink;
their logical widths must equal the saved candidate costs exactly. Width-only
results contain Unknown ink and cannot be used as renderable local layouts.

### Proposed narrow DP rounding decision

**Recursive comparison itself is normative**, rather than exact global
squared-slack minimization or global enumeration followed by a rounded-cost tie.
Process starts backwards. A state's candidates are only an admissible edge plus
the one already selected plan at its end. Never reopen a discarded suffix when
a preceding large cost erases a former score difference. Compare candidates by
line count, rounded binary64 score, mode-specific width vector and decoded break
offset vector, in that order. Terminal cost is positive zero. Use binary64
round-to-nearest/ties-to-even separately for subtraction, square multiplication
and addition; no FMA, extended-precision score, epsilon or arbitrary-precision
global objective. A nonfinite score is LimitExceeded, not an alternative wrap.

The append-only adversarial experiments use available width16384 and
delta=2^-15. Suffix A has exact cost2^-30 and suffix B cost4×2^-30. Adding a
zero-width preceding line costs2^28 and rounds both totals to2^28. Mode0 and
mode3 can each prefer B in a global rounded comparison, but recursive DP must
keep previously selected A. A second fixture has two rounded-equal complete
scores, exact2^28+2^-30 versus2^28: mode3 selects the former by its width tie,
explicitly demonstrating that exact global minimization is not promised.
These fixtures check the actual experimental search, with independently
calculated multiplication/bit expectations; existing small-plan assertions stay
unchanged. Future production code must spell out separate square multiplication,
not rely on a general power function's unspecified precision.

## Proposed work bounds and required measurement

W1's **10,000 candidates / 1,000,000 total shaped glyphs per event** are an
experimental starting point, not accepted defaults. The additive
`wrapping_workload_review` experiment enumerates independently authored whole
word/ideograph candidate ranges using actual pinned Latin, Arabic, Devanagari
and Japanese fonts. It records candidate count, actual glyph work, runtime,
peak per-candidate conservative reservation and the exact stopping reason for
ordinary and long typesetting workloads. This is workload evidence, not an
implemented Unicode breaker or UAX14 conformance result. A separate 60-second
experiment budget is reported distinctly from the proposed production limits.
Raw results and any proposed default adjustment are presented on PR #29 before
requesting W2 approval. No production limit/algorithm is changed by the experiment.

### Actual workload evidence — defaults remain unapproved

Completed on candidate `7900332ff318a4fb029add11cbc9f063d1eafdcd`,
[Linux native job118040887111](https://github.com/amanosatosi/taiyou/actions/runs/39246785396/job/118040887111).
[Original JSON records](../../experiments/t017/results/7900332/wrapping-workloads.jsonl)
and adjacent provenance preserve the actual values. This used the existing
multilingual fixtures' explicit 512 MiB font-intake configuration; the default
128 MiB intake correctly rejected the large CJK font in the earlier experiment.
No production budget was increased.

| Workload | Candidates evaluated / all | Glyphs shaped | Seconds | Actual stop |
|---|---:|---:|---:|---|
| Latin12 words | 78/78 | 2,616 | 0.728 | complete |
| Arabic16 words | 136/136 | 6,176 | 1.175 | complete |
| Devanagari12 words | 78/78 | 2,106 | 1.439 | complete |
| Japanese36 ideographs | 136/666 | 2,387 | 60.116 | experiment-time-budget |
| Latin204 words | 1,377/20,910 | 1,000,847 | 15.797 | glyph-work-limit |
| Japanese225 ideographs | 136/25,425 | 9,316 | 60.137 | experiment-time-budget |

Thus 10,000/1,000,000 are **not validated production defaults**. The CJK stops
are incomplete searches, not accepted wrapping results. The experimental glyph
counter is checked after each whole candidate; its observed 847-glyph overshoot
is reported. A production implementation must reserve remaining work before
native calls or reject the completed candidate/event, never publish a partial
wrap or treat this experimental counter as a strict native-work bound.

Concrete next gate: independently measure a candidate-width prepass using the
existing logical advances, spacing, bidi and fallback policy without collecting
glyph ink/outline bounds for every rejected candidate. Selected lines still
need full boundary shaping and ink metrics. First prove advance/source-range
equivalence against the full shaping API on these workloads plus mixed styles,
joining scripts, clusters and negative/fractional advances; then remeasure all
candidates and propose exact caller limits. This is a proposed bounded experiment,
not authorization to introduce new font metrics, a cache or a production shortcut.
If that equivalence/cost cannot be demonstrated, automatic modes remain explicitly
unsupported rather than silently switching to a generic wrapper. Owner approval
of W2 semantics alone must not be represented as approval of these unverified
work defaults or of Legacy behavior.

An accepted implementation must expose caller-selected candidate/glyph-work
limits in Limits/C7. Count every actual candidate shaping, including repeats;
preflight work where possible. Crossing a limit returns LimitExceeded, preserves
diagnostics/counters and produces no partial/greedy substitute. Peak candidate
tables/retained chosen shapes stay under existing transient limits, checked
before allocation. Per-call memoization may avoid repeated shape work but is
charged and released in that call. No acceptance claim for arbitrary 1-MiB
typesetting events follows from a few representative measurements.

## Completed optimized-width experiment (production admission pending)

Candidate `ae467c953b9cf937469fb118e48f3f8c909fcaac`,
[Linux job118108262024](https://github.com/amanosatosi/taiyou/actions/runs/39266748578/job/118108262024),
[original records](../../experiments/t017/results/ae467c9/width/T017-width-prepass.jsonl).
The later sanitizer executable-discovery failure does not erase these completed
measurements. These are debug-test timings, including selected independent full
line checks, not optimized playback or production wrapping measurements.

| Workload | Candidates / all | Returned glyphs | Seconds | Independent full-line checks | Result |
|---|---:|---:|---:|---:|---|
| Latin12 words | 78/78 | 2,616 | 0.041 | 78 | complete |
| Arabic16 words | 136/136 | 6,176 | 0.083 | 136 | complete |
| Devanagari12 words | 78/78 | 2,106 | 0.204 | 78 | complete |
| Japanese36 ideographs | 666/666 | 8,436 | 0.249 | 666 | complete |
| Latin204 words | 20,910/20,910 | 10,507,802 | 27.673 | 25 | complete |
| Japanese225 ideographs | 25,425/25,425 | 1,923,825 | 7.429 | 29 | complete |

Ordinary CJK preparation/shaping/extent costs were 6.290/93.826/2.575 ms;
long CJK 275.678/6838.702/214.175 ms. Long Latin shaping took 26.301 s;
preparation 366.104 ms and logical extent calculation 820.740 ms. Boundary
shaping now dominates; rebuilding ASS and evaluation per candidate is unnecessary.
No full LocalLayout is allocated for rejected candidates. The stored cost
matrices occupied 1,248–406,800 logical bytes, checked within the experimental
caller budget; these bytes do not represent total native scratch or process RSS.

The separate release twelve-range CJK profile records parsing/evaluation/full
shaping and repeated font-authentication costs in the original adjacent
`optimized-initial/*-cjk-profile.stderr.txt` reports. That controlled experiment
holds the rebuilt ASS harness constant while removing repeated authentication.
It therefore distinguishes the expensive native/font work from the harness's
small parsing/evaluation cost. Glyph/advance/offset identity is checked across
all three implementations. The full boundary equivalence matrix additionally
covers fallback, mixed styles, bidi, joining scripts, combining/ZWJ sequences,
original source ranges and positive/fractional/negative spacing on all native
platforms. Long searches verify selected boundaries, not all 46,335 long costs.

**Limit recommendation for further review:** 32,000 candidates and 16,000,000
returned glyphs cover every completed authored workload above. They are sensible
pilot limits, not validated production defaults. The glyph counter currently
checks after each completed candidate and does not count every fallback attempt,
rejected native glyph or cluster-context reshape. A strict production native-work
budget must count those calls and reserve/reject remaining work before publishing
an event. Unicode conformance, generated-table hashes/size/license and a bounded
raw-native-work counter are still missing. Consequently no exact C7 defaults or
normative limit amendment is admitted here. Exceeding any approved caller limit
must fail deterministically with no partial frame or greedy fallback.

The additive search experiment already compares the DAG/DP proposal against
exhaustive small nonmonotonic plans, including opposite ties, overwide atoms and
mode1 scanning beyond a nonfitting end. It is not necessary to repeat complete
plan enumeration or build a production wrapper to resolve this policy decision.
The owner approved W2 semantics and the Unicode15.1 data plan in principle.
Final review now concerns the narrow recursive-rounding clarification in
draft ADR-013; pilot budgets and production admission remain gated on the
explicitly missing conformance/native-work evidence and final contract approval.

## Executable proposed arithmetic expectations

Synthetic advances: Latin/space=1; CJK ideograph=2; punctuation=1;
ascent8/descent2. No font pixel result is invented by these arithmetic examples.

| Text / width | Modes | Expected selected lines |
|---|---|---|
| aa bb cc / 6 | 1,0 | aa bb / cc (widths5,2) |
| aa bb cc / 6 | 3 | aa / bb cc (widths2,5) |
| seven leading spaces + a / 6 | 0,1,3 | one 8-unit overwide atom |
| a + seven trailing spaces / 6 | 0,1,3 | one 8-unit overwide atom |
| seven leading spaces + a + space + b / 6 | 0,1,3 | leading-spaces+a / b |
| a + space + b + seven trailing spaces / 6 | 0,1,3 | a / b+trailing-spaces |
| eight spaces only / 6 | 0,1,3 | one full-metric width8 line |
| aa + seven interior spaces + bb / 6 | 0,1,3 | aa / bb; delimiter consumed |
| aaaa NBSP bbbb / 6 | 0,1,3 | one width9 overwide atom |
| 日本語字幕 / 6 | 1,0 | 日本語 / 字幕 (widths6,4) |
| 日本語字幕 / 6 | 3 | 日本 / 語字幕 (widths4,6) |
| 「日本」 / 5 | 0,1,3 | 「日 / 本」; never isolate opening/closing punctuation |
| a+combining acute+b / 1 | 0,1,3 | one unbreakable atom; never cut the combining sequence |
| aa hard-N hard-N bb / 6 | all | aa / empty / bb, total height30 |
| aa soft-n bb hard-N cc / 6 | 0,1,3 | aa bb / cc |
| aa soft-n bb hard-N cc / 6 | 2 | existing aa / bb / cc |

Additional independent boundary fixtures must include RI flags, emoji ZWJ,
nonbreaking glue, Hangul syllables/Jamo, Arabic joining, Indic conjuncts,
punctuation, numeric groups, long no-break words and source intervals crossing
overrides. Use Unicode conformance data for opportunities, and separately pinned
font shaping for widths/context. Test multiple style runs, actual newly shaped
Arabic forms, fractional ties and empty lines. Unicode opportunities alone do
not establish line metrics, raster pixels or historical compatibility.

## Approval gate

Owner review approved Core semantics, the DP direction conditionally and the
Unicode15.1 data plan in principle. Final confirmation of the narrow recursive
rounding decision and [proposed ADR/C4 text](../decisions/013-core-unicode-wrapping.md)
is still required. Generated data hashes/licensing/provenance/storage review,
unmodified Unicode conformance, separate ASS-tailored/cluster-filtered fixtures,
raw native-work preflight and a reviewed C7 amendment remain mandatory before
implementation admission. The32,000/16,000,000 pilot limits are not production
defaults. No automatic production wrap mode is enabled. Legacy admission separately needs authenticated pinterf
L01–L03 captures, font/GDI metrics and tie examples with the full environment/
binary/font/script/timestamp/output provenance specified by R001. Neither this
Core proposal nor libass output supplies that historical evidence.
