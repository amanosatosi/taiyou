# T017 adaptive staging and W2 performance follow-up

Work in progress following reviewed candidate
`027e58a52efdd870b99f49ab5f3bae6e952f8f59`. Existing measurements, including the
5,000-event four-glyph 180-second timeout, remain evidence, not acceptance limits.
W2 remains proposed; no automatic wrapping mode is enabled.

## Deterministic staging

Eligibility examines parsed operations only: the evaluator starts with a default
anchor and only SetPosition writes that anchor. Style resets and unsupported or
recovered literal operations do not reset it. Actual evaluation still enforces
strict/tolerant policy. Renderer event references avoid repeating the public
linear EventId lookup.

For two or more eligible events, retain a stable prefix of prepared immutable
layouts up to one eighth of the initial remaining transient budget. Account
the slot array, transitive local ownership and saved diagnostics separately.
Unretained events retain only collision descriptors and are reconstructed during
painting. Explicit events never enter this preparation or collision indexing.
Zero/one eligible events have no collision prepass. Slots are sorted by EventId,
looked up in logarithmic time and taken without moving other slots.

Optional retention cannot change a successful minimal frame into LimitExceeded:
if an attempt using optional storage hits a limit or allocation failure, drop
the entire unpublished attempt and retry once with minimal staging. No output
lease, diagnostics, masks, native context or retained layout survives the failed
attempt. Genuine minimal failures remain failures. This is bounded frame-local
staging, not a cache, eviction framework or memory-observation heuristic.

The initial one-quarter trial is preserved as partial evidence at candidate
`2574e17e1c60ef106e3785947f327c6c0f75313b`: a 32 MiB, 256-event/32-glyph
effects scene replayed after admission, producing 640 shaping calls versus
512 for minimal staging. The smaller admission fraction leaves more native
scratch headroom. A new 32 MiB test requires successful partial reuse with fewer
preparations than minimal staging, preventing a replay from masquerading as
successful partial retention. Whole-frame retry still protects arbitrary tiny
caller budgets.

## Font authentication and experimental widths

The immutable validated catalog already authenticates its font bytes. New
catalog-backed outline/shaping paths reuse that proof; public untrusted blob
outline extraction still authenticates each supplied blob. All native dimension,
face, glyph, scratch and resource checks remain. No mutable native face is cached.

The W2 advances experiment shares actual itemization, HarfBuzz shaping, fallback,
cluster/source ownership and metrics with full shaping. It skips native glyph ink
extraction and deliberately publishes Unknown ink, preventing its use as ordinary
renderable layout. Candidate widths still require real boundary-specific shaping;
no approximate character widths or monotonicity assumption is introduced.

## Verification

New work has not yet completed Actions verification. Final paired measurements,
resource evidence, failure analysis and owner decisions will be recorded here.
F1 remains approved Core policy; W2 remains unapproved. PR #29 stays unmerged.
