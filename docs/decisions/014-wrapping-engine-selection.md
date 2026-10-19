# ADR-014 — Proposed event-level wrapping-engine selection

Status: **proposed contract amendment; separate owner review required**.
Date: 2026-10-20. Task branch: `task/T017-warpmode-contract`.
Trusted integration base: `d5a545ac49aabbfc23e3895231d407da576e276f`.
Prerequisite: PR29, reviewed head `3144993e31e52513e044f47a95518930e0a45a80`,
accepted by the owner as a **partial T017 collision/backend checkpoint** subject
to its exact human baseline acknowledgment. It was still open with no issue
acknowledgment comment at inspection. This branch neither merges nor modifies it.
Reconcile this proposal against its actual accepted merge before implementation.

## Request and scope

The owner requests a Taiyou-specific override spelled exactly `\warpmode`, with
default0 selecting xy-VSFilter-style wrapping intent and explicit1 opting into
the approved ADR-013 Unicode algorithm. The existing `\q`/WrapStyle axis stays
independent. This is an explicit extension, not an upstream xy tag or automatic
profile switch. The engine choice does not admit LegacyXyV1 rules, GDI metrics,
codepage behavior, stateful placement or a different raster backend.

This proposal changes C4 and ADR-013's previous **exclusive** Core automatic
wrapping selection; the approved Unicode algorithm remains valid. Numeric
tailoring, backwards recursive binary64 comparison and deterministic ties are
unchanged. No parser/evaluator/renderer/CLI code, public IR, runtime dependency,
production limit, protected fixture or support classification changes here.
Proposed inputs/expectations are in [the fixture manifest](../../experiments/warpmode-contract/cases.json),
not accepted output images or actual xy captures. No fixture is promoted to a
protected suite by this document or its future parser tests.

## Proposed selector contract

1. The tag name is case-sensitive ASCII `warpmode`. Its complete parameter is
   the single ASCII digit0 or1, allowing only ASCII space/tab around that digit.
   There is no empty reset, sign, decimal/exponent, suffix, locale conversion,
   nonfinite value, clamping or prefix acceptance. Thus2,10,-1,1.0,NaN and an
   empty parameter are invalid;1foo is not a valid1. An unknown different tag,
   such as `\warpmodes1` or `\WarpMode1`, follows existing unknown-tag policy.
   Preserve Core's supported-name disambiguation: an alphabetic extension of the
   name is unknown, except recognized nonfinite numeric spellings, which are
   malformed parameters. A numeric-leading suffix such as1foo is a malformed
   complete parameter, never a valid1 prefix. Review this deliberately finite
   grammar before adding a typed operation.
2. Start each event with selector0. Resolve **the last valid standalone parsed
   operation** in original event source order. A valid late selector applies to
   every paragraph, including earlier text. Do not split shaping/paint runs just
   because the selector appeared in another override block. Resolve once before
   paragraph preparation and collision layout; never rescan raw substrings.
3. Bare `\r` and valid named resets change style under the existing contract
   but leave the selector unchanged. So do font/paint changes, hard/soft breaks,
   blank paragraphs and event layers. A new event starts at0; no history or
   previous-event selector is retained. No style/header default for warpmode is
   introduced. Explicit `\warpmode0` is the way to return to the default engine.
4. Animation/transforms do not animate this event-level selector. A candidate
   selector inside an unsupported/nested operation is not standalone and never
   executes. Preserve existing outer-operation strict/tolerant behavior; do not
   extract its children. Literal text outside an override block cannot select an
   engine. Malformed-block recovery remains the existing Core parser policy;
   do not change it merely to recognize this extension.
5. Strict malformed supported parameters return InvalidScript at the original
   operation span. Recovery emits the existing bounded malformed-record
   diagnostic and preserves the preceding valid selector; an invalid later tag
   is not implicit0. Preserve every operation's original span, including losing
   selectors. The winner has its exact source span; default0 records absence,
   not a fabricated source range. Typed host operations must validate0/1 and
   source ownership through the normal immutable Track ingestion boundary.

These are proposed new **Core extension** rules. In particular last-wins and
reset persistence are not asserted as historical xy behavior. Legacy parsing
and any recognition of this extension in LegacyXyV1 require separate admission;
using the Core Unicode engine must never silently switch the whole profile.

## Two independent axes

| q style | Selected0: xy-style intent | Selected1: ADR-013 Unicode |
|---|---|---|
|0|R001-L03 source-observed xy smart/top-biased policy; capture/metrics gate|Approved recursive binary64 mode0, wider-earlier tie|
|1|R001-L02 source-observed whitespace-group policy; capture/metrics gate|Approved furthest-fitting-end mode1, no monotonic-width assumption|
|2|Existing Core hard-break path; no automatic engine invoked|The same hard-break path; no automatic engine invoked|
|3|R001-L03 source-observed xy alternative balancing; capture/metrics gate|Approved recursive binary64 mode3, narrower-earlier tie|

Missing selector always means0; it never changes a supplied q style. Missing q
retains the script WrapStyle, including Core's existing default2. Selection does
not map q0 to q1, swap q0/q3, turn q2 into automatic wrapping or change q reset,
escape or precedence rules. `\N` remains mandatory; q2 preserves the existing
soft-break behavior, full style metrics for empty lines and nonbreaking `\h`.

Actual main and PR29 **do not implement `\q` overrides**: the supported parser
name list omits q, the evaluator takes script WrapStyle and rejects non2. This
proposal must not pretend otherwise. It defines the engine axis, not a rewrite
of q evaluation. Repeated/mid-text q changes need a precise separately reviewed
evaluation rule before enabling those configurations. In the local xy source,
ParseString interprets soft breaks while successive text fragments are parsed;
a late q operation cannot be casually applied retroactively to those fragments.
Uniform-q fixtures below avoid inventing that unresolved behavior. Script-level
q0/1/2/3 and uniform tag equivalents need independent admission tests.

## Verification gates and failure semantics

There is no production engine capability registry in this change. Both automatic
engines remain disabled. Selector0 needs authenticated R001-L01–L03 captures,
identified real pinterf binary/font/Windows environment, measured word/line
metrics, exact break/tie examples and human admission. The local source is the
only xy implementation reference, not a rendering oracle. Selector1 needs every
ADR-013 gate: pinned Unicode15.1 UAX14rev51 complete Regex-Number/LB13 profile and
UAX29rev43 data/license/storage review, unmodified conformance separately from
ASS/grapheme/shaping-cluster filters, full boundary-specific shaping/fallback/
bidi/provenance, all raw native-work accounting and preflight, approved C7
candidate/native/storage limits, deterministic finite DP and integration tests.
The32000/16M pilot limits are not production defaults. No default changes here.

Proposed failure rule: resolve the final selector and q requirement before
checking the selected engine's admission. If q0/1/3 requires an unavailable
engine, return typed UnsupportedFeature under **both** strict and tolerant
rendering. Do not try the other engine, use a guessed unwrapped layout, skip an
event or publish a partial frame/diagnostic list. The winner's span (or genuine
default/header provenance) identifies the failure. No inactive event forces a
gate failure; existing no-active-event clear-frame behavior remains intact.
Existing unsupported tags elsewhere retain their own policy and errors; engine
selection is not permission to ignore them. q2 invokes an already supported
shared path, not fallback, even while both automatic engines are gated.

This strict/tolerant structural-failure rule requires explicit review. Current
tolerant handling of unsupported features is not silently changed by this
proposal. Parser recognition, selector evaluation, engine admission and renderer
activation are distinct milestones; recognizing1 must not advertise a working
automatic engine.

## Stage, resource and provenance obligations

Preserve parsed operation ordering and ownership. Evaluate selection without
font lookup or shaping, then freeze it for the whole event. Preserve source maps
through consumed delimiters, mandatory/blank lines, style seams, fallback and
bidi clusters. Measure candidates under actual line-boundary shaping; neither
engine can substitute additive character widths. Never borrow the other engine's
candidate costs, legal boundaries, metrics or failed search state.

Whole-event engine selection and q style enter local-layout policy identity.
Boundary-shape reuse additionally includes actual range/context/font/fallback
and original source identity, as already required by K2. Changing selector may
change breaks, line baselines and collision footprints, so rebuild complete
active-set placement; it does not authorize reshaping an unchanged glyph merely
for event translation. Preserve immutable outputs and approved F1 after-layout
footprints when PR29 is integrated. No pixel padding, history cache or T019
framework is added. There is one engine per event, even across multiple paragraphs.

Use checked caller-budget accounting and deterministic preflight before native
work/allocation; no worker-count-dependent failures, nonfinite scoring, silent
greedy fallback or partially published frame. Record engine/policy revision, q,
winning-source provenance and resource configuration in future stage evidence.
No new bounds are accepted here. Engine selection must not increase repeated
evaluation/shaping of the already reviewed collision/backend pipeline.

## Source evidence and limits

Inspected the read-only local pinterf pfmod3.2.0.810-labelled tree from the local
reference map. The24 entry-point fingerprint check passed. RTS.cpp SHA-256 is
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`;
STS.cpp is `59f431e3ac36d99e2b694cdab106a530edaf7bbc8c85410f91647d3ff4578895`.
The supplied archive's equivalence to the locked upstream commit is unverified.
No source was refetched, copied or translated into a Taiyou implementation.

| Local RTS.cpp function/range | Finding | Evidence status |
|---|---|---|
|CSubtitle::GetWrapWidth1421–1450|Distinct0/3 width heuristic,1 unchanged width,2 no auto constraint|Source-observed; not capture/metric acceptance|
|CSubtitle::GetNextLine1453–1503|Whitespace/nonwhitespace groups and overwide-first-group behavior|Source-observed; exact line decisions pending|
|CRenderedTextSubtitle::ParseString1966–2020|N/n/h treatment uses current subtitle wrap state while decoding text fragments|Source-observed; mixed-q behavior needs explicit fixtures|
|CRenderedTextSubtitle::ParseSSATag2612–2625|q writes separate subtitle wrap state; r writes style|Source-observed; not authority for the new Taiyou selector|
|CRenderedTextSubtitle::GetSubtitle2872–2949|Initialize wrap from script, process override/text fragments in source order|Source-observed; render/binary identity unverified|

No authentic xy capture, exact break result or pixel-compatibility claim is made.
Libass remains the independent protected suite and cannot establish xy behavior.
R001's proposals and future authenticated xy suite remain separate and unmodified.

## Admission sequence and review request

1. Human acknowledgment and integration of PR29; keep its acceptance partial.
2. Review this selector/reset/grammar/failure amendment and reconcile C4/ADR-013
   against that accepted merge. No dependent parser/evaluator implementation
   before the contract review required by AGENTS.md.
3. Add typed bounded tag recognition/selection and actual parser/source-span/
   strict/recovery/nesting tests, preserving q's separate admission boundary.
4. Satisfy and review each automatic engine's own evidence/resource gates before
   activating it. Complete Core Unicode integration only under explicitly
   accepted C7 limits; do not await or fabricate xy evidence for Unicode testing.
5. Run applicable units/native/sanitizer/platform checks and full immutable
   external comparison on implementation candidates, preserving genuine exit
   status/artifacts. New independent fixtures remain additive. Record exact
   per-engine supported/blocked status; no full-T017 completion claim.

Owner decision requested: accept the finite parameter grammar, last-valid
event-wide selection (including late tags), reset persistence, uniform-q
independence, q2 shared path, and no-fallback UnsupportedFeature behavior above,
or amend their exact fixture expectations. This is separate from C7 defaults,
engine activation, baseline acknowledgment or merge approval.
