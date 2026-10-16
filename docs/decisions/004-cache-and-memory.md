# ADR-004: Typed caches and budgeted ownership

Status: Accepted. Date: 2026-10-17.

## Problem

Correct reuse must account for fonts, transforms, phase and effects, without an unbounded shared cache or duplicate expensive misses.

## Alternatives

One frame-result cache; Per-worker independent caches; General concurrent LRU framework; Typed stores with coordinator admission.

## Decision

Typed stage keys, shared byte budget, immutable Arcs, unique-owner allocation charges, barrier eviction and one producer per key per frame.

## Reasons

Keys explain invalidation to agents; coordinator protocol avoids lock-heavy work and saturated-pool single-flight deadlocks.

## Tradeoffs and risks

Evicted but pinned allocations remain charged; native internal memory is only measured initially. Conservative keys may miss reuse.

## Revisit when

Measured coordinator lookup cost dominates or concurrent frame rendering becomes a real host requirement; preserve semantics and full-key equality.

## Implementation gate

Tasks: T019/T024. See [contract and evidence](../CACHING.md). No unmeasured performance claim accompanies this decision.
