# Implementation task index

T001 and T003 are integrated, and the owner accepted ADR-008 and the integrated T003 parser baseline. Other tasks remain planned until their implementation and acceptance evidence is recorded. Cargo commands run on Actions only. Read the task's linked contracts before implementation; source-established ASS behavior is normal implementation work; narrow research is reserved for genuinely ambiguous questions. Owner acceptance is not independent-review or T005 protection evidence.

**Verification runs in GitHub Actions.** Do not compile or install build dependencies on the user's PC. All Cargo/native/test commands below target CI runners; local work is editing and lightweight documentation/hash checks. Read the [execution policy](../DEVELOPMENT.md#d0-execution-location-and-local-storage), provide candidate SHA and Actions run/job links, and report unrun checks as pending. T001 supplies the initial remote source-check lane; this policy does not mean workflows already exist or have passed.

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
| [T016](T016.md) | Borders and shadows | Sol 6.1 | T014 |
| [T017](T017.md) | xy wrapping and deterministic collision | Sol 6.1 | T010, T016 |
| [T018](T018.md) | Bounded scalar blur and edge blur | Sol 6.1 | T016 |
| [T019](T019.md) | Typed caches and accountable memory | Sol 6.1; architect key review | T017, T018 |
| [T020](T020.md) | Move, fades and numeric transforms | Sol 6.1 | T019 |
| [T021](T021.md) | ASS vector drawings and independent clipping | Sol 6.1 | T011, T018 |
| [T022](T022.md) | Cluster-aware basic karaoke | Sol 6.1; typography review | T020, T009 |
| [R004](R004.md) | Resolve ambiguous complex transform/aspect behavior | Astra | T011 |
| [T023](T023.md) | Approved 3D transforms and aspect mapping | Sol 6.1 | R004, T019 |
| [T026](T026.md) | Common SSA and explicit decoding support | Sol 6.1 | T006, T008 |
| [T027](T027.md) | Usable single-worker headless renderer integration | Sol 6.1 | T019, T020, T021, T022, T023, T026 |
| [T029](T029.md) | Reproducible KFX and performance corpus | Sol 6.1 | T027 |
| [R003](R003.md) | Measure scheduling, memory and tile granularity | Astra | T029 |
| [T024](T024.md) | Bounded parallel preparation and unique-key jobs | Sol 6.1; Astra concurrency review | R003, T027 |
| [T025](T025.md) | Deterministic tile composition | Sol 6.1 | T024, T013 |
| [T028](T028.md) | C ABI v1 and lifetime validation | Sol 6.1; Astra ABI/unsafe review | T025, T027 |
| [T030](T030.md) | Release verification and platform hardening | Sol 6.1 integration; Astra release review | T028, T005, T029 |
| [T031](T031.md) | Extension lowering research after the usable release | Astra planning; Sol isolated experiments | T030 |
