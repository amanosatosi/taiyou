# ADR-001: Immutable stages and geometry/paint separation

Status: Accepted. Date: 2026-10-17.

## Problem

Complex KFX repeatedly changes colors and placement while reusing glyph geometry. A monolithic mutable event renderer makes reuse and parallel ownership difficult.

## Alternatives

A libass-shaped mutable render context; A generic render graph/plugin framework; Concrete immutable stage artifacts.

## Decision

Concrete immutable Track, evaluated runs, shaped/local layouts, paths, A8 masks and ordered draw operations; keep one rendering crate with private stage modules.

## Reasons

Stable artifacts make independent agents and caches tractable; color/alpha can change without rebuilding outlines. The coordinator owns transactions and ordering.

## Tradeoffs and risks

More explicit intermediate objects and Arc traffic; avoid one allocation per scalar or glyph. Paint-dependent grouping can still invalidate grouped masks.

## Revisit when

Measured intermediate allocation cost dominates even after arena/batch improvements, or a supported feature cannot lower into these stages.

## Implementation gate

Tasks: T003/T019. See [contract and evidence](../CONTRACTS.md). No unmeasured performance claim accompanies this decision.
