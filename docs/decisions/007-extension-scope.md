# ADR-007: Preserve seams without speculative extension machinery

Status: Accepted. Date: 2026-10-17.

## Problem

Gradients, ruby, curves, distortion and future formats need space in the architecture but can delay basic ASS support indefinitely.

## Alternatives

Implement all extensions now; Bake solid color into glyph bitmaps; Maintain concrete seams and defer syntax.

## Decision

Separate paint, coverage passes, glyph placement and geometry mappings. Study libassmod/Mangetsu after the compatibility release; no generic plugin/DSL system.

## Reasons

Each extension has a named lowering stage and can reuse existing masks/outlines. Fundamental milestones stay small.

## Tradeoffs and risks

Some future feature may require a contract revision, especially isolation groups or complex annotation layout. No unknown .asa syntax is promised.

## Revisit when

A concrete authoritative extension specification and fixtures demonstrate a missing representation; add only that capability through an ADR.

## Implementation gate

Tasks: T031. See [contract and evidence](../ARCHITECTURE.md). No unmeasured performance claim accompanies this decision.
