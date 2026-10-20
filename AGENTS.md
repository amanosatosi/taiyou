# Instructions for Taiyou coding agents

1. Read your task, linked contracts, and relevant ADRs before editing. All unimplemented paths are planned. Implement one assigned task, not the entire roadmap.
2. `docs/CONTRACTS.md` is normative for interfaces; `docs/COMPATIBILITY.md` for behavior; `docs/CACHING.md` for reuse. Keep dependencies acyclic. Core contract changes require a separate ADR and architect review before dependent implementation.
3. When a task conflicts with a contract, stop the conflicting portion, record the smallest concrete conflict and proposed choices, and continue independent authorized work. Never invent an undocumented compatibility rule.
4. Preserve worker-count determinism, event order, integer blending and immutable stage outputs. No shared mutable font faces or schedule-dependent visual decisions.
5. No silent compatibility changes. Give every change focused regression coverage and evidence. Use **pinterf/xy-VSFilter** as the main and only xy-VSFilter reference; other historical renderers are separately named evidence. For established ASS/SSA behavior, inspect pinterf, implement the observed behavior directly, test it and compare it. Research is an exception for genuinely ambiguous source behavior, not an admission gate.
   Read reference sources locally first: see [local reference map](docs/LOCAL_REFERENCES.md) for `C:\taiyou-time\refrences`. Use `rg` and local file reads; do not repeatedly browse GitHub, fetch, clone or pull available sources. Online links in the blueprint are provenance, not instructions to refetch. Report missing sources or revision mismatches; go online only when the task specifically requires unavailable material or a requested update. Treat reference trees as read-only, and never infer a Git commit from an archive folder name.
6. Never rewrite reference outputs, widen tolerances, skip tests, reduce the supported set, or alter trusted validation to make an implementation pass. Report unsupported cases and failures separately. Baseline changes follow `docs/TESTING.md`.
7. Use safe, explicit Rust interfaces. Confine justified unsafe code to reviewed native adapters, FFI and measured SIMD kernels. Check dimensions and allocation arithmetic; do not panic on malformed subtitles.
8. Add no dependency without recording version, license, purpose and alternatives. No speculative optimization, new framework, plugin architecture or unrelated refactoring.
9. Work on `task/<ID>-<slug>` from the agreed integration revision. One writer per file set; serialize shared contracts and lockfiles. Commit focused changes when Git is established; never reset user changes, force-push or merge your own protected changes.
10. Run the task's checks and applicable fast suite. Never report unrun tests as passed. Report commit/base, files, tests, failed/unsupported cases, limitations and benchmark evidence where relevant. A compile-only result does not complete a rendering task.
11. **CI-first; no local compilation.** The user's PC has insufficient storage for builds. Run Cargo/compiler commands, native dependency builds, renderer tests, fuzzing, sanitizers and benchmark executables in GitHub Actions, not on this PC. Do not install build toolchains, download build dependencies/large corpora or create local build caches. Local work is editing, reading existing references, Git and lightweight documentation/hash checks. Include the tested commit and Actions run/job links in completion reports. Missing CI is a verification gap to fix or report, never permission to compile locally. See [execution policy](docs/DEVELOPMENT.md#d0-execution-location-and-local-storage).

See [development](docs/DEVELOPMENT.md) for model assignment and [guidelines](docs/GUIDELINES.md) for implementation standards. No renderer exists at blueprint delivery; do not claim otherwise.

### Protected comparison tests — NEVER MODIFY TO MAKE CI PASS

**NEVER modify, delete, replace, weaken, skip, reduce or disable existing comparison tests to make an implementation pass.**

**NEVER change comparison thresholds, expected images, frozen corpus references, case selection, support classifications or failure interpretation to hide rendering differences.**

**NEVER replace a full regression run with a focused subset. Focused tests are supplemental only.**

**NEVER remove a previously tested regression case, silently narrow workflow triggers or exclude a failing platform.**

**NEVER regenerate or overwrite reference images simply because Taiyou renders different pixels.**

**NEVER classify infrastructure failures, missing fonts, crashes or unsupported behavior as successful visual matches.**

**NEVER change the existing comparison test harness or regression policy without an explicit, separately reviewed maintainer instruction. If a legitimate task requires such a change, STOP and request authorization before editing.**

For ordinary renderer tasks, the existing comparison suite is read-only.

Taiyou will maintain two independent protected regression suites: the existing
pinned libass-tests references and future authenticated pinterf xy-VSFilter
output captures. The xy suite is **PLANNED, not operational**; it requires
authenticated build/environment/output provenance and explicit maintainer
acceptance before use. Once properly established and accepted, it must never
replace or weaken the libass suite, and the libass suite must never replace or
weaken it. Neither suite's tests, references, coverage, thresholds, classifications
or failure interpretation may be altered to hide failures. Keep their evidence
and results separate. This policy does not implement the xy suite or authorize
inventing captures.

Fix the renderer, parser, layout, rasterizer or font implementation instead of changing the tests.

New supplemental regression tests may be proposed, but existing coverage must remain intact. Any new tests or validation mechanisms must preserve the immutable reference policy and receive appropriate review.

The maintainer's current CI coverage restoration instruction explicitly authorizes
restoring CI coverage, NOT modifying existing comparison algorithms, oracle
fixtures or accepted outcomes. It is a separate protected CI proposal under
[ADR-011](docs/decisions/011-solo-maintainer-validation.md), not standing permission
for future tasks. After final technical review, the human owner must post the
new exact-head/base/change-digest acknowledgment; agents must not post it.

Every agent, including Sol, Luna and any future coding agent, must:

1. Read this policy before editing any rendering feature.
2. Run the existing applicable unit and stage tests in GitHub Actions.
3. Execute the full external comparison through GitHub Actions for applicable PRs; focused tests are additional smoke tests only.
4. Report exact visual matches, unsupported visual frames, infrastructure errors, regressions and crashes separately; report parser-only safety probes and upstream-disabled sources separately from visual matches.
5. Explain every newly failing case using its exact ID, failure evidence and responsible implementation or infrastructure gap.
6. Never alter tests to avoid addressing an implementation defect.

Preserve all inventory entries and frozen corpus assets. Genuine rendering or
infrastructure improvements may change observed result counts; never hardcode
today's outcome counts as permanent acceptance requirements. A red full comparison
must remain red, with its original report and difference artifacts uploaded even
after failure. Required Corpus policy gate, Fast gate, Platform gate and
Documentation structure checks and branch protection remain intact.
