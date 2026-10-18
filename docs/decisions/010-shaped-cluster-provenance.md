# ADR-010: Decoded cluster ranges with retained original-source maps

Status: **Accepted as architectural direction by explicit repository-owner decision in Session 4.** Provisional dependent T009 implementation is authorized against this ADR. This records owner-level architectural review, not a submitted independent GitHub approval. Both PRs remain unmerged for final technical review.

## Concrete conflict

C4 currently declares `ShapedGlyph.cluster: SourceSpan`. In `f{\c&HFF0000&}i`, a Core fi ligature consumes the original f and i intervals but not the intervening override bytes. One continuous original-source span cannot represent those intervals exactly. The same problem occurs for a combining sequence crossing a paint boundary. Session 4 explicitly requires noncontiguous provenance, and rejects a fabricated bounding span.

Separately, C2's unit `HardBreak`, `SoftBreak` and `NoBreakSpace` variants carry no token span. T007 therefore cannot recover exact escape positions. PR 17's checked `SourceMappedText` does not itself supply missing positions. Rescanning decoded strings, guessing from event spans, or reparsing source independently in the shaper would not fix the ownership contract.

## Proposed minimal amendment

1. Keep `TextOp` variants and `parse_ass` unchanged. Add an optional immutable per-operation span sidecar to Event. The parser supplies the original span for each operation as it tokenizes; a host can explicitly supply spans through a checked publication path. Existing host-created unit operations remain valid IR with unknown provenance. Track publication checks sidecar length, source containment/order, literal equality and the exact two bytes for each supplied escape. Replacing an event program clears its old sidecar.
2. Move the pure `SourceMappedText` implementation to taiyou-types, retaining the `taiyou_font::source_map` re-export for compatibility. The seven-package DAG is unchanged. The evaluator builds one checked immutable map from known text/escape pieces, retains it on the evaluated event, and records each evaluated run's decoded range. Its existing `source()` remains diagnostic provenance, not an exact escape/cluster claim. An event with an unmappable text operation has no complete map; ordinary evaluation still works, but exact shaping rejects missing provenance. Never invent offsets for host-created spanless operations.
3. Introduce checked `DecodedSpan` (half-open UTF-8 byte range in a particular retained decoded text), distinct from original `SourceSpan`. Replace only the planned, not-yet-implemented C4 glyph cluster field:

```rust
struct ShapedGlyph {
    font: FontId, glyph: u32, cluster: DecodedSpan,
    advance_x: FiniteF64, advance_y: FiniteF64,
    offset_x: FiniteF64, offset_y: FiniteF64,
}
struct ShapedRun {
    glyphs: Arc<[ShapedGlyph]>, metrics: RunMetrics, direction: Direction,
    source_map: Arc<SourceMappedText>, decoded: DecodedSpan,
}
```

Every glyph's cluster range is checked within its run and against that run's immutable map. Resolving original provenance returns **all** original intervals via the map; there is no bounding-span shortcut. Shaped results retain the map and source after input buffers, track, evaluated event and native contexts are dropped. No native pointer escapes.
4. Paint-only boundaries remain outside shape itemization. The first logical scalar's paint applies to a crossing cluster per existing C4, using the cluster's decoded start to locate the existing evaluated paint span. Hard breaks retain decoded/source ranges and delimit shaping paragraphs; this adds no line layout or visual run reordering.

## Alternatives

- Store an owned original-span array on every glyph. Precise but duplicates intervals and ownership across repeated glyphs in one cluster; it still needs the tokenizer/evaluator sidecar.
- Replace C2 unit variants with span-bearing variants. Precise but breaks existing host construction and tokenizer tests. A sidecar preserves that interface and distinguishes unknown host provenance explicitly.
- Use the original interval hull. Rejected: includes override bytes that did not contribute text and violates the Session 4 requirement.

## Migration and verification

No executable C2/C3/C4 amendment is implemented by this policy PR. The owner explicitly accepted the decoded-range/retained-map design and parser span sidecar, and authorized the necessary C2/C3/C4 updates and provisional dependent parser/evaluator/shaper implementation. Final integration remains separately review-gated; owner acceptance is not an independent GitHub approval or T005 protection.

The owner's architectural review additionally requires: validate supplied provenance against the actual event text field whenever known, preserving operation ordering, source identity and exact escape bytes; and check every DecodedSpan against its owning decoded text/map, rejecting cross-map confusion. Existing TextOp construction remains compatible and host-created spanless units remain explicitly unmappable. These requirements apply to the candidate, not just a later integration.

Preserve all existing tokenizer, evaluator and source-map regressions. Add parser-to-evaluator-to-shaped tests for all escapes, UTF-8/BOM/CRLF, fi and combining clusters crossing override/paint gaps, producer lifetime, invalid supplied sidecars and explicitly unmappable host-created units. Preflight sidecar/map/glyph allocations using existing C7 limits. Keep the native unsafe boundary, reference pins, expected outputs and seven-package DAG unchanged.

## Independent work while pending

The private native design-unit shaping adapter, Unicode itemization, deterministic catalog fallback and provisional upstream HarfBuzz numeric-fixture acquisition do not depend on the amended source-provenance publication contract. Those may be implemented and tested independently, with raw HarfBuzz clusters explicitly labeled as decoded UTF-8 offsets. Do not publish them as compliant original-source clusters or claim full T009 acceptance while this gate is pending.
