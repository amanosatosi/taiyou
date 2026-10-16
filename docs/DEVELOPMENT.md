# Agent development and CI

## D1. Roles and merge ownership

**Astra:** owns contracts/ADRs, R001–R004 decisions, cross-profile behavior, high-risk native/FFI/concurrency reviews and difficult performance analysis. **Sol 6.1:** implements scoped stage modules and integration with explicit acceptance criteria. **Luna 6:** implements deterministic parsing helpers, value types, scalar arithmetic, manifests and narrowly scoped tests where no architectural decision is required. These are task assignments, not claims that a model's output is inherently correct.

One task branch from a known integration commit; one writer per file set. Foundation/contracts merge before dependent branches start. A coordinator records task ID, owner, base SHA, leased files, dependencies and state in an issue/project board. Agents may inspect each other's branches but must not independently change shared types, Cargo.lock, feature flags or CI policy. Coordinator integrates in dependency order and reruns affected checks after rebasing. No simultaneous shared-worktree editing during implementation.

Use `task/<ID>-<slug>` branches and focused commits. Never force-push shared branches. Separate architectural/policy PRs from implementation PRs; when a contract changes, architect gives migration steps and updates all affected task prerequisites. A blocked agent supplies a minimal failing case and exact ambiguity, not a redesign. New tasks follow the task schema and cannot be considered accepted merely because they compile.

## D2. Safe parallel lanes

After T003: parser T004, corpus infrastructure T005 and font catalog T008 can proceed concurrently in separate file sets (T008 also needs T002). After T009/T010: geometry tasks and comparison-adapter tasks can run alongside each other. After the vertical slice: isolated scalar paint tests, cache-key tests and source research may proceed separately. T016/T018 share coverage/effect integration and should merge sequentially. T024 must wait for proven single-worker stage behavior and R003. T025 follows T024. T028 ABI work waits for public Rust API stabilization.

Serialize: `taiyou-types`, contracts/ADRs, native dependency recipes, lockfile updates, capability policies, reference promotion and render coordinator integration. Do not delegate one shared enum to several agents. Integration itself is a named task with a clear owner and failure triage, not “everyone fixes the build”.

## D3. GitHub Actions design

Planned workflows (T001 creates only docs checks; later tasks establish these):

| Workflow | Trigger / cost control | Required outputs |
|---|---|---|
| `blueprint.yml` | Every PR, cheap; no Rust setup for docs-only changes | link/schema/task-DAG checks |
| `fast.yml` | Source/manifest changes; one Linux toolchain | fmt, clippy, unit tests, safe API checks; fail fast |
| `focused.yml` | After fast succeeds; trusted changed-module map | relevant stage/golden/determinism results |
| `compatibility.yml` | Dedicated dispatch/schedule/milestones | full corpus and all status counts |
| `platforms.yml` | Native/FFI/build changes, milestones and release | Windows MSVC, Linux, macOS ARM64; additional x86/ARM coverage per release support |
| `safety.yml` | Nightly/dispatch, changed native surface | fuzz smoke, sanitizer and pure-Rust Miri lanes |
| `bench.yml` | Maintainer dispatch on controlled runner | raw before/after results; never ordinary hosted timing gate |

Use job-level conditions and an always-running gate aggregator so documentation-only PRs do not leave required checks pending. Path changes to selectors/workflows trigger policy review; they cannot disable their own gate. Rust source changes in types/font/evaluate/raster/compose or native versions select all dependent cases; unknown classification selects the broader suite. Schedule/full validation catches missing coverage mappings.

Cache dependency downloads and native build artifacts by OS/arch/compiler version/flags, Cargo.lock and native source hashes. Cache Rust build artifacts only with compatible keys; untrusted candidate jobs cannot publish cache entries consumed by privileged validation. Cancel superseded PR jobs, cap matrices, use concurrency groups and upload only bounded failure artifacts plus compact summaries. Source compilation happens once per target where possible; focused comparison reuses that binary. Expensive cross-platform full corpora wait for milestone gates.

GitHub permissions default read-only, actions pinned by full commit SHA, no secrets for candidate code, no untrusted forks on persistent benchmark runners. Protected runner/harness design is [V4](TESTING.md#v4-protected-validation-trust-model). The current empty local directory has no GitHub remote or organization policy, so this blueprint does not claim these protections are active.

## D4. Acceptance and cost control

An implementation task normally fits one focused session. If its preconditions expose an unresolved rule, stop that slice and open a bounded research item; do not spend days guessing. A coordinator may split a task while preserving its ID as an umbrella and declaring child scopes. Prefer local fast checks followed by one focused CI run; full matrices are not necessary for prose changes. Do not buy speed by omitting meaningful verification or hiding failures.

Architect review is required for new native/unsafe boundaries, accepted external differences, budget/rounding changes, cache dependency reductions and output reordering. Routine private refactoring within a task is permitted only if necessary for its objective. Every completion report follows [guidelines](GUIDELINES.md#required-report).
