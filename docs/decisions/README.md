# Architecture decisions

Engineering decisions retain their API, ownership, safety and resource safeguards.
Historical behavioral proposals are evidence, not barriers to verified ASS/SSA
corrections. Update descriptive compatibility notes when findings change; an
ordinary rendering correction does not require another ADR.

* [ADR-001: Immutable stages and geometry/paint separation](001-immutable-stages.md)
* [ADR-002: Mature native typography with a bounded raster experiment](002-font-and-raster.md)
* [ADR-003: Ordered coordination with bounded parallel work](003-deterministic-parallelism.md)
* [ADR-004: Typed caches and budgeted ownership](004-cache-and-memory.md)
* [ADR-005: Versioned behavioral authority and protected evidence](005-profiles-and-references.md)
* [ADR-006: Owned premultiplied RGBA output and opaque C handles](006-output-and-ffi.md)
* [ADR-007: Preserve seams without speculative extension machinery](007-extension-scope.md)
* [ADR-008: Explicit parser ingestion ownership](008-track-ingestion-identity.md)
* [ADR-009: Core reset-token disambiguation (owner-accepted; protected integration pending)](009-core-reset-token-disambiguation.md)
* [ADR-010: Decoded cluster ranges and retained original-source maps (proposed)](010-shaped-cluster-provenance.md)
* [ADR-011: Solo-maintainer validation and explicit baseline promotion (maintainer-authorized)](011-solo-maintainer-validation.md)
* [ADR-012: Retired Core geometric collision proposal](012-core-collision-footprint.md)
* [ADR-013: Core Unicode wrapping and recursive score selection (owner-approved contract; production admission pending)](013-core-unicode-wrapping.md)

* [ADR-014: Event-level wrapping-engine selection and source-derived mode0](014-wrapping-engine-selection.md)
