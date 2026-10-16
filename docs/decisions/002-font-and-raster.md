# ADR-002: Mature native typography with a bounded raster experiment

Status: Accepted baseline; compatibility backend gated. Date: 2026-10-17.

## Problem

Implementing font parsing/shaping is expensive and risky, while native GDI behavior does not trivially match a cross-platform font stack.

## Alternatives

Custom Rust shaping/font parser; Platform-specific GDI/DirectWrite/CoreText shaping; HarfBuzz + FreeType behind narrow Rust adapters; Pure Rust stack after maintenance/compatibility review.

## Decision

HarfBuzz shapes from immutable font bytes; FreeType extracts unhinted outlines and provides initial grayscale raster/stroker support. Worker-local native contexts. Rust Unicode analysis. R002 chooses whether a separate compatibility coverage algorithm is needed.

## Reasons

Uses mature algorithms, confines native complexity and preserves cross-platform stage contracts. FreeType recommends per-thread library objects for simple thread isolation (see SOURCES references and its library-setup documentation).

## Tradeoffs and risks

Native packaging, FFI safety, version-dependent output and GDI metric differences. FreeType is not presumed xy-pixel-compatible. License/version choices require T002.

## Revisit when

R002 proves unacceptable error on the approved supported suite, native packaging becomes unsustainable, or a maintained Rust alternative passes identical feature/performance/license gates.

## Implementation gate

Tasks: T002/T008/T009/T012/R002. See [contract and evidence](../SOURCES.md). No unmeasured performance claim accompanies this decision.
