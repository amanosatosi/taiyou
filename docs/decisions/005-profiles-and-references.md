# ADR-005: Versioned behavioral authority and protected evidence

Status: Accepted. Date: 2026-10-17.

## Problem

libass, original VSFilter and xy variants disagree. A test suite can be made green by silently changing baselines or support labels.

## Alternatives

Treat libass output as universal truth; Promise all-engine pixel equivalence; Versioned profiles and independent immutable reference suites.

## Decision

CoreV1 development profile; LegacyXyV1 supported families based only on pinned pinterf xy evidence. Separate original/libass reports. Trusted validation policy and maintainer-controlled baseline promotion.

## Reasons

Enables honest incremental development without forcing agents to resolve undocumented behavior or hide unsupported cases.

## Tradeoffs and risks

Legacy research/captures require Windows environment and maintainer time. Protected CI needs real repository settings not yet available.

## Revisit when

New authoritative evidence justifies a new profile revision or separately scoped historical profile, with before/after captures and review.

## Implementation gate

Tasks: T005/T015/R001. See [contract and evidence](../COMPATIBILITY.md). No unmeasured performance claim accompanies this decision.
