# T017 W2 — Unicode-aware Core wrapping proposal

Status: **proposed, not approved or implemented**. Supersedes W1's space-only
proposal following the owner's 2026-10-19 review. F1 is separately approved in
[ADR-012](../decisions/012-core-collision-footprint.md). This proposal does not
admit R001 L01–L03 or claim historical xy-VSFilter behavior.

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
- Modes0/3 minimize line count, then sum of squared slack
  max(0,available_width-line_width)^2. Equal-cost mode0 chooses the
  lexicographically larger line-width vector; mode3 chooses smaller. Final tie
  is lexicographically earliest decoded break-offset vector. Evaluate costs in
  fixed logical order, with no epsilon or unspecified hash order.

Publish only selected line shapes, their visual run order and full provenance.
Use T010 max ascent/descent including empty-line style metrics. Build distinct
logical/ink/local boxes after wrapping, then ADR-012 effects footprint, then
complete-active-set collision placement. No frame-history or retained wrap cache.

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

An accepted implementation must expose caller-selected candidate/glyph-work
limits in Limits/C7. Count every actual candidate shaping, including repeats;
preflight work where possible. Crossing a limit returns LimitExceeded, preserves
diagnostics/counters and produces no partial/greedy substitute. Peak candidate
tables/retained chosen shapes stay under existing transient limits, checked
before allocation. Per-call memoization may avoid repeated shape work but is
charged and released in that call. No acceptance claim for arbitrary 1-MiB
typesetting events follows from a few representative measurements.

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

Owner approval is still required for this exact policy, its measured search
limits, dependency/data choice and C4/C7 amendment. No automatic production wrap
mode is enabled yet. Legacy admission separately needs authenticated pinterf
L01–L03 captures, font/GDI metrics and tie examples with the full environment/
binary/font/script/timestamp/output provenance specified by R001. Neither this
Core proposal nor libass output supplies that historical evidence.
