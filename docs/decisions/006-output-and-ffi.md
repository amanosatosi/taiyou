# ADR-006: Owned premultiplied RGBA output and opaque C handles

Status: Accepted design; ABI freeze at T028. Date: 2026-10-17.

## Problem

Host integration needs predictable memory lifetime and color semantics; exposing Rust layout or transient glyph bitmap lists would be fragile.

## Alternatives

Expose libass-compatible linked bitmap lists; Borrow output until next frame; Owned RGBA frame and stable C handle API.

## Decision

Owned encoded-domain premultiplied RGBA8 frames, exact integer source-over, explicit stride/size, opaque C handles and versioned size-prefixed structs.

## Reasons

Paint extensions naturally compose into RGBA. Frame remains valid across render calls and renderer destruction. Hosts can integrate without Rust ABI knowledge.

## Tradeoffs and risks

Full-frame output bandwidth and premultiplication round trips; host owns YUV conversion. Sparse output may be useful later but is not an initial ABI promise.

## Revisit when

Measured full-frame copy/bandwidth dominates real host workloads; add a versioned sparse/tile view without breaking existing lifetime guarantees.

## Implementation gate

Tasks: T013/T028. See [contract and evidence](../CONTRACTS.md). No unmeasured performance claim accompanies this decision.
