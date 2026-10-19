# Warpmode contract fixtures — proposed only

These29 independently authored UTF-8 ASS inputs distinguish event-level engine
selection, resets, late/repeated tags, invalid/recovered parameters, nested
unsupported operations and literal text. The last input has two independent
events. There are no accepted images, fabricated captures or renderer results.
[cases.json](cases.json) records proposed typed-state/line/error expectations;
[input receipts](inputs.sha256.json) pin exact independently authored script bytes.

Review [ADR-014](../../docs/decisions/014-wrapping-engine-selection.md) before
implementation. Tags are **not supported by the current public parser**. q-tag
inputs are uniform within each event; this proposal does not invent mixed-q
precedence or retroactive escape decoding. q2 expectations preserve full style
empty-line metrics and the current hard/soft/NBSP behavior, without asserting
font-dependent pixels. Uniform q0/1/3 inputs expect a clear unavailable-engine
error until their chosen engine is admitted, not a rendered unwrapped fallback.

Font identity is the existing latin entry in
[shaping-fonts.proposed.json](../../build-support/shaping-fonts.proposed.json).
No font bytes are downloaded or redistributed here. Font-based future stage
tests must acquire/verify that pin on Actions, record actual metrics/output and
retain complete source maps. These selector fixtures are not xy evidence.
Authenticated pinterf wrapping captures require separate scripts/environment
provenance and review; never replace the independent libass suite with them.

Future executable tests must exercise the actual admitted tokenizer, typed Track
ingestion, event selector and evaluator, not a raw-text substring scanner or a
second toy parser. Assert original byte spans for every winning/losing/invalid
tag, including CRLF/BOM and decoded-offset cases; strict versus recovery and
unknown versus malformed must remain distinct. Preserve style/paint/source maps
across reset/late tags. Add font-based same-engine seek/collision equivalence,
mixed positioned events, tiny memory/native-work budgets and no-partial-frame
failures after each engine is ready. Existing protected tests remain unchanged.

All actual build/test/render execution is GitHub Actions only. This contract
proposal's Actions checks exercise unchanged main code and protected comparison,
not an implementation of these proposed selector expectations. A passing build
does not turn this manifest into accepted runtime or historical evidence.
