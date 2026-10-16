# ADR-003: Ordered coordination with bounded parallel work

Status: Accepted. Date: 2026-10-17.

## Problem

Parallel alpha composition and collision decisions can change output, and small subtitle jobs can cost more to schedule than to render.

## Alternatives

Global thread pool and per-glyph spawn; Fully sequential renderer; Renderer-owned pool with stage barriers and independent tiles.

## Decision

Single-worker oracle first. Later Rayon pool with configurable workers, coordinator deduplication, immutable inputs, serial collision and disjoint tile composition in stable order.

## Reasons

Preserves arithmetic/order while allowing expensive independent preparation and pixel work to scale. No nested pools or worker waits on dependent misses.

## Tradeoffs and risks

Barriers and serial collision limit speedup. Frame latency can be dominated by one large mask. More threads increase scratch needs.

## Revisit when

Controlled evidence shows barriers dominate or large masks need internal parallel kernels; change requires identical deterministic tests and bounded memory.

## Implementation gate

Tasks: R003/T024/T025. See [contract and evidence](../ARCHITECTURE.md). No unmeasured performance claim accompanies this decision.
