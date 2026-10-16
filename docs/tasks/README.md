# Implementation task index

All tasks are **planned**, not completed. Filenames mentioned inside tasks are planned unless already delivered by this blueprint. Cargo commands are contracts for the future workspace, not commands that can run today. Read the task's linked contracts before implementation; an unresolved research prerequisite is a real gate.

For every source-study task, use the [local reference map](../LOCAL_REFERENCES.md) first. Read the supplied archives directly; upstream URLs elsewhere in tasks/documents are provenance, not requests to browse again. Do not infer archive-to-commit equivalence or replace immutable oracle pins.

Assign **[T001](T001.md)** first. T001 establishes the workspace and cheap checks. After it, T002, T003 and inventory preparation in T005 can proceed in disjoint file sets; shared manifests/lockfiles still have one owner. [Roadmap](../ROADMAP.md) determines milestone order; this table's numeric order is not a topological execution order.

Every task requires its Objective, Prerequisites, Files and modules, Technical contract, Expected behavior, Implementation guidance, Acceptance criteria, Verification, Forbidden changes and Completion report sections. Research tasks have bounded sessions and concrete decision outputs. Acceptance cannot be replaced by a build-only report.

| ID | Task | Suggested owner | Prerequisites |
|---|---|---|---|
| [T001](T001.md) | Repository and workspace foundation | Luna 6 | None |
| [T002](T002.md) | Freeze dependency and native build recipes | Sol 6.1; Astra review | T001 |
| [T003](T003.md) | Typed units, errors and immutable IR subset | Luna 6; shared interface review | T001 |
| [T004](T004.md) | Minimal ASS sections, styles and event records | Luna 6 | T003 |
| [T005](T005.md) | Pinned corpus inventory and protected validation bootstrap | Sol 6.1; maintainer configuration | T001 |
| [T006](T006.md) | Bounded override tokenization | Luna 6 | T004 |
| [T007](T007.md) | Static tag evaluation and active event selection | Sol 6.1 | T006 |
| [T008](T008.md) | Font catalog and native context ownership | Sol 6.1; unsafe review | T002, T003 |
| [T009](T009.md) | Itemization, fallback and shaping | Sol 6.1; typography review | T008, T007 |
| [T010](T010.md) | Hard-break layout and alignment | Sol 6.1 | T009 |
| [T011](T011.md) | Owned glyph paths and 2D geometry mapping | Sol 6.1 | T010 |
| [T012](T012.md) | Scalar baseline fill raster adapter | Sol 6.1; native review | T011 |
| [T013](T013.md) | Scalar paint and ordered RGBA composition | Luna 6 | T003 |
| [T014](T014.md) | Single-worker vertical slice CLI | Sol 6.1 | T007, T010, T012, T013 |
| [T015](T015.md) | Reference adapters and full comparison reports | Sol 6.1 | T005, T014 |
| [R001](R001.md) | Freeze the first legacy behavior profile | Astra; Sol may collect evidence | T015 |
| [R002](R002.md) | Choose compatible coverage, stroke and blur policies | Astra; Sol experiment assistance | R001, T012 |
| [T016](T016.md) | Borders and shadows for the approved subset | Sol 6.1 | R002, T014 |
| [T017](T017.md) | Approved wrapping and deterministic collision | Sol 6.1 | R001, T010, T016 |
| [T018](T018.md) | Bounded scalar blur and edge blur | Sol 6.1 | R002, T016 |
| [T019](T019.md) | Typed caches and accountable memory | Sol 6.1; architect key review | T017, T018 |
| [T020](T020.md) | Move, fades and numeric transforms | Sol 6.1 | T019, R001 |
| [T021](T021.md) | ASS vector drawings and independent clipping | Sol 6.1 | T011, T018, R001 |
| [T022](T022.md) | Cluster-aware basic karaoke | Sol 6.1; typography review | T020, T009 |
| [R004](R004.md) | Freeze complex transform and aspect policies | Astra | R001, T011 |
| [T023](T023.md) | Approved 3D transforms and aspect mapping | Sol 6.1 | R004, T019 |
| [T026](T026.md) | Common SSA and explicit decoding support | Sol 6.1 | R001, T006, T008 |
| [T027](T027.md) | Usable single-worker headless renderer integration | Sol 6.1 | T019, T020, T021, T022, T023, T026 |
| [T029](T029.md) | Reproducible KFX and performance corpus | Sol 6.1 | T027 |
| [R003](R003.md) | Measure scheduling, memory and tile granularity | Astra | T029 |
| [T024](T024.md) | Bounded parallel preparation and unique-key jobs | Sol 6.1; Astra concurrency review | R003, T027 |
| [T025](T025.md) | Deterministic tile composition | Sol 6.1 | T024, T013 |
| [T028](T028.md) | C ABI v1 and lifetime validation | Sol 6.1; Astra ABI/unsafe review | T025, T027 |
| [T030](T030.md) | Release verification and platform hardening | Sol 6.1 integration; Astra release review | T028, T005, T029 |
| [T031](T031.md) | Extension lowering research after the usable release | Astra planning; Sol isolated experiments | T030 |
