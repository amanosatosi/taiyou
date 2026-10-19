# Warpmode selector proposal inputs

These29 independently authored UTF-8 ASS inputs are the historical selector
proposal, with original input receipts retained. They are not accepted reference
images, xy captures, or the implementation's test registry.
The old manifest's automatic-engine-unavailable expectations describe that earlier
selector-only stage; they do not describe the working mode0 implementation.

Actual executable coverage is in
`crates/taiyou-cli/tests/warpmode.rs` and
`crates/taiyou-cli/tests/automatic_wrapping.rs`, plus the private xy_wrap units.
These test parsing, source spans, recovery, event-level last-wins/reset semantics,
source-ordered q conversion and escapes, q0/1/2/3 line decisions, inherited and
event margins, overwide words, real rendering, and fresh/repeated/seek equality.
The existing pinned Source Sans3 fixture is acquired only on Actions.

Mode0 uses the local pinterf word-fitting policy with Taiyou's HarfBuzz widths.
Mode1 automatic wrapping remains gated; q2 uses the unchanged shared path.
See [the implementation record](../../docs/decisions/014-wrapping-engine-selection.md)
for exact source functions, identities, limitations and PR29 integration dependency.
No Legacy capability, historical metric/pixel equivalence, protected reference,
C7 default or runtime dependency is admitted by these inputs.
