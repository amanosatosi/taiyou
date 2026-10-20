# ADR-013 — Core Unicode wrapping and recursive score selection

Status: **owner-approved contract; production admission pending**.
The human owner's explicit PR29 implementation-session confirmation accepts
this exact recursive binary64 decision and the Unicode15.1 numeric profile below,
and authorizes the C4 amendment. The data plan is approved in principle subject
to provenance, licensing, conformance and storage review. This approval does not
enable production wrapping, establish C7 defaults or admit LegacyXyV1.
ADR-012/F1 remains separately accepted. The final exact-head ADR-011 baseline
acknowledgment remains a separate human decision.

## Explicit Unicode opt-in amendment

The approved algorithm below is the `\warpmode1` engine. Missing `\warpmode`
or explicit0 selects source-derived xy-style word wrapping, as requested by the
owner and recorded in [ADR-014](014-wrapping-engine-selection.md). q0/1/2/3 is
independent; q2 uses the shared hard-break path. This changes selection only,
not the approved Unicode algorithm, profile, binary64 recurrence or gates.
Mode1 automatic wrapping remains unavailable pending its outstanding prerequisites
and C7 review; mode0 never silently replaces a requested mode1. PR29 merged as
a partial collision/backend checkpoint at
`6cfa33ddb38c714a43a096abf53096f9f9aff69c`; preserve its approved F1 policy.
No full T017 completion or Legacy admission is implied.

## Decision

Adopt the exact opportunities, atoms, admissibility, boundary shaping, source
provenance, margins, mandatory/soft breaks and mode selection rules in
[W2](../research/T017-wrapping-policy-v2.md). Contract policy identity:
`core-wrap-unicode15_1-recursive-1`. Existing WrapStyle2 is unchanged.

For modes0/3, **the recursive DP comparison is normative**. At each logical start,
consider only each admissible edge followed by the previously selected suffix
at that edge's end. Select by line count, binary64 score, opposite mode0/3 width
vector ties, then earliest decoded break-offset vector. Process starts backwards
with the terminal plan's score positive zero. Candidate widths remain actual
boundary-specific shaping results; no monotonic-width assumption is allowed.

Calculate nonnegative slack from available width and line width. Round binary64
subtraction, square multiplication and addition separately to nearest/ties-even.
The score is `square(slack(edge)) + selected_suffix.score`, not a reassociated
sum, FMA, extended-precision accumulator or exact rational global objective.
Reject nonfinite arithmetic with LimitExceeded. Do not reopen a discarded suffix
when a large preceding score erases its former difference. The recursive choice
can differ from globally comparing all rounded complete-plan scores, and can
also differ from exact global squared-slack minimization. Both differences are
explicit policy, not accidental implementation behavior.

This choice preserves bounded reusable candidate costs and one selected suffix
per state. Exact global optimization would require a different arithmetic/search
policy and resource analysis. It is an alternative requiring owner direction,
not an undocumented promise of this recursive policy.

## Executable arithmetic expectations

The additive `wrapping_search_experiment` tests exercise the actual experimental
search at available width16384 and delta2^-15. Two suffixes have costs2^-30 and
4×2^-30, while a zero-width first line contributes2^28. Both complete scores
round to2^28. The locally lower-cost suffix must remain selected even when each
opposite mode's global lexicographic comparison would prefer the discarded one.

A second example has two lines with widths(0,16384-delta) or(0,16384). Both
binary64 scores round to2^28; mode3 chooses the smaller second width although its
exact score is larger by2^-30. Mode0 chooses the other path. These fixtures make
the distinction from exact global minimization observable. Existing small random
plan tests use exactly representable small costs; their exhaustive agreement is
preserved without pretending it proves arbitrary floating global optimality.

The pilot's constant-exponent square is checked against independently calculated
multiplication/bit expectations in these fixtures. Production admission must use
the explicitly specified separate multiplication. No production wrapper exists.

## Authorized C4 amendment

The owner authorized replacing C4's sentence “T017 introduces other wrap modes
only after the standard xy behavior is source-verified” with the following text, preserving the rest of the
WrapStyle2/line-metrics/box-separation paragraph:

> CoreV1 automatic WrapStyle0/1/3 follows ADR-013 and W2's pinned Unicode15.1
> opportunities, Core tailoring, grapheme/shaping-cluster filters, overwide atoms,
> inherited margin width and fresh boundary shaping with original provenance.
> The line profile is Unicode15.1 UAX14 revision51 section8.2 Example7's complete
> Regex-Number behavior with its required LB13 adjustments, not the default LB25
> approximation; grapheme boundaries use UAX29 revision43.
> Mode1 examines all legal ends without assuming monotonic widths. Modes0/3 use
> the normative backwards recursive binary64 DP comparator, including opposite
> width-vector ties and earliest decoded offsets; discarded suffixes are never
> reopened after prefix rounding. This is not exact global squared-slack
> minimization. Preserve explicit empty-line metrics and separate logical/ink/
> collision boxes; apply ADR-012 footprints and complete-active-set placement
> after chosen lines are fully shaped. Production automatic modes remain
> UnsupportedFeature until the data/conformance/native-work prerequisites and
> separately reviewed C7 caller limits are satisfied. Legacy automatic wrapping
> still requires authenticated R001 evidence and separate admission.

The amendment records the approved contract only. It does not add a runtime
policy revision or approve C7 defaults. Production activation remains subject
to every prerequisite below and separately reviewed resource limits.

## Mandatory implementation prerequisites

### Approved Unicode numeric profile

The unmodified Unicode15.1 LineBreakTest header explicitly selects UAX14
section8.2 Example7 numeric tailoring, rather than the default LB25 pair
approximation. The isolated prerequisite probe tests that exact declared profile
and keeps the default pair path separate. Core W2 uses this numeric profile,
together with LB1's default SA/CJ resolution. Use the complete Regex-Number
behavior and its required LB13 adjustments; do not mix the default LB25
approximation into this profile. For example `%(` has no
interior break under default pair rules, but has one under the upstream test
profile when no following number exists. Pin the profile explicitly, run the
original unmodified LineBreakTest against its declared numeric tailoring, and
test default pair rules separately, including this distinction. ASS-specific
tailoring and shaping-cluster restrictions require independent tests; upstream
conformance must never be adjusted to hide those differences.

* Vendor/hash Unicode15.1 UAX14 revision51 and UAX29 revision43 properties and
  matching unmodified conformance inputs; record license/provenance/table size.
  No new runtime dependency is proposed. Host Unicode tables are not accepted.
* Run unmodified Unicode conformance separately from ASS tailoring and cluster
  filtering. Do not modify expected Unicode boundaries to match Core tailoring.
* Retain full line-shaping/source/bidi/fallback equivalence; count all native work,
  including fallback attempts and cluster-context reshapes, with checked counters
  and preflight enforcement. Fail without publishing a partial frame.
* Review exact caller-configurable C7 search/native-work/storage limits. Pilot
  32,000-candidate/16,000,000-returned-glyph bounds remain experimental; completed
  workloads alone do not validate native-work defaults.
* Preserve all protected libass comparisons and the independent planned xy suite.
  No new reference capture, expected image, tolerance or classification is admitted.

The [complete performance report](../../experiments/t017/results/77afbd2/summary.md)
and [W2 width evidence](../research/T017-wrapping-policy-v2.md#completed-optimized-width-experiment-production-admission-pending)
inform feasibility only. PR#29 stays unmerged and T017 automatic wrapping remains
incomplete until these gates are met.
