# ADR-008: Explicit parser ingestion ownership

Status: **Accepted by explicit repository-owner architectural decision.** Date: 2026-10-17. The owner accepted the explicit ingestion-owner argument and the integrated T003 interface as the parser baseline in the autonomous development session. This records owner-level acceptance, not independent third-party review or T005 protection evidence.

## Problem

C1 requires renderer-assigned, session-scoped TrackRevision values. C2 returns an already identified immutable Track from `parse_ass(bytes, options)`, while C10 ParseOptions carries only encoding, profile, limits and syntax. T003 requires a documented ingestion owner that never recycles identities in a renderer session. The current parser signature provides no route for that owner. See [issue 3](https://github.com/amanosatosi/taiyou/issues/3).

## Decision

Add one explicit argument to the planned parser interface:

```rust
fn parse_ass(bytes: &[u8], options: &ParseOptions, ingestion: &mut TrackIngestor)
    -> Result<ParsedTrack, ParseError>;
```

The renderer/session coordinator owns exactly one non-cloneable TrackIngestor and uses it for every track admitted to that session. The parser validates and normalizes records, then asks that owner to publish the immutable Track and allocate its next revision. Failed record/source validation does not allocate an identity. Once allocated, an identity is never reused, including after a later failure or track destruction. Exhaustion returns LimitExceeded. Cloning a snapshot preserves its identity and immutable storage; editing publishes a new snapshot.

TrackIngestor is mutable coordinator state, not a shared stage output. Parsing through one owner is serialized by Rust's exclusive borrow. Separate owners belong to separate sessions; their revision numbers must not be mixed in a renderer's caches. No global counter, pointer-derived identity, hidden callback, shared mutable Track or new ParseOptions field is introduced.

## Alternatives

Keep the two-argument signature and add an owner to ParseOptions: this mixes mutable session state with a value configuration and requires additional borrowing/interior-mutability decisions. Allocate globally: this changes the stated owner and namespace. Return an unidentified draft and identify it later: this changes the Track/ParsedTrack representation and publication boundary. The explicit argument changes the smallest interface surface before any parser exists.

## Migration and scope

No parser or external API consumer exists on the integration base `1c43f3c33097ab905771f28c87ae2740927a558d`. [T003 PR 4](https://github.com/amanosatosi/taiyou/pull/4) proposes the ingestion owner and immutable records; it does not implement a parser. C1 identity meaning, C10 ParseOptions fields, arithmetic, defaults, profiles, references and cache rules remain unchanged.

## Implementation gate

PRs 4 (T003) and 5 (the contract) are integrated. The owner explicitly accepted this decision and approved the integrated T003 interface as the parser baseline. T004 may use that baseline. Further core-interface changes still require their own reviewed decision; owner acceptance here does not establish independent review or the protections required by T005.
