# Agent development and CI

## D0. Execution location and local storage

The repository is now initialized in Git and has `origin` at `https://github.com/amanosatosi/taiyou.git`. The user has uploaded it to GitHub. **GitHub Actions is the primary and required execution environment for compilation and renderer validation.** The local PC cannot accommodate compilation storage; this is an ongoing project constraint, not a temporary missing-toolchain problem.

| Work | Execution location |
|---|---|
| Editing, Git inspection/commits, reading existing reference trees, small text/JSON/documentation checks and selected-file hashing | Local PC is permitted |
| Cargo commands (including check/test/run/clippy/fmt/metadata/tree), Rust/C/C++ compilation, native library builds, dependency resolution and lockfile generation | GitHub Actions runners |
| Unit/integration/image/determinism tests, CLI/C ABI smoke, fuzzing, sanitizers, Miri and performance smoke | GitHub Actions runners |
| Definitive performance measurements | Actions-dispatched controlled runner, provisioned separately; never assume the user's PC is available as a runner |

Do not install Rust/native SDKs, start Docker/VM build environments, fetch build dependencies or full test corpora, create `target/` or local native build trees, or download large CI artifacts onto this PC. Keep existing local sources read-only for inspection. Small logs and individual failure reports may be inspected locally; leave full binaries, symbol files, corpus archives and image collections in CI storage. Do not delete the user's existing files to make room.

Verification commands in task files are **runner commands** unless explicitly identified as lightweight local checks. Lack of a local compiler is expected and is not a blocker to authoring a change. Add or repair the appropriate focused workflow as part of the task, following the protected validation rules. Run on the exact candidate commit through the authorized branch/PR or dispatch path; inspect job logs and resolve failures. Do not publish a change solely because the remote exists; use the session's Git publishing authorization. If CI cannot be triggered or is not yet implemented, report verification pending and the exact missing prerequisite. Do not substitute local compilation or claim success from writing a workflow.

Completion evidence includes candidate SHA, workflow/run URL, relevant job names, commands, pass/fail/unsupported results and small artifact links. A queued/skipped/cancelled run is not a pass. Subsequent code changes need validation of the new SHA. T001 must establish both cheap documentation CI and a minimal source-check lane so its own workspace/CLI checks can run remotely; later tasks expand that lane. T002 generates dependency locks and validates native recipes on runners, returning small lockfiles/reports as artifacts for reviewed inclusion in the repository.

Actions cannot read `C:\taiyou-time\refrences` on this PC. Provision required sources/fonts on the runner from verified pinned origins or approved archive storage, verify hashes, and cache them there. This is an explicit CI acquisition step and does not change the local-first rule for source study. Never assume the unversioned local archives equal the upstream pins. The full external corpus inventory remains T005 work.

Keep hosted-runner timing as performance smoke only. If controlled hardware for R003 is unavailable, leave that measurement gate open; do not run it on this PC or invent benchmark evidence.

## D1. Roles and merge ownership

**Astra:** owns contracts/ADRs, R003/R004 narrow research when actually needed, cross-profile behavior, high-risk native/FFI/concurrency reviews and difficult performance analysis. **Sol 6.1:** implements scoped stage modules and integration with explicit acceptance criteria. **Luna 6:** implements deterministic parsing helpers, value types, scalar arithmetic, manifests and narrowly scoped tests where no architectural decision is required. These are task assignments, not claims that a model's output is inherently correct.

T005 validation governance follows owner-authorized [ADR-011](decisions/011-solo-maintainer-validation.md): the sole human maintainer accepts baseline/infrastructure changes and exact implementation candidates. Independent multi-maintainer validation is optional; ordinary CI is not described as independently trusted. Agents must not post human baseline acknowledgments or self-merge protected proposals.

One task branch from a known integration commit; one writer per file set. Foundation/contracts merge before dependent branches start. A coordinator records task ID, owner, base SHA, leased files, dependencies and state in an issue/project board. Agents may inspect each other's branches but must not independently change shared types, Cargo.lock, feature flags or CI policy. Coordinator integrates in dependency order and reruns affected checks after rebasing. No simultaneous shared-worktree editing during implementation.

Use `task/<ID>-<slug>` branches and focused commits. Never force-push shared branches. Separate architectural/policy PRs from implementation PRs; when a contract changes, architect gives migration steps and updates all affected task prerequisites. A blocked agent supplies a minimal failing case and exact ambiguity, not a redesign. New tasks follow the task schema and cannot be considered accepted merely because they compile.

## D2. Safe parallel lanes

After T003: parser T004, corpus infrastructure T005 and font catalog T008 can proceed concurrently in separate file sets (T008 also needs T002). After T009/T010: geometry tasks and comparison-adapter tasks can run alongside each other. After the vertical slice: isolated scalar paint tests, cache-key tests and source research may proceed separately. T016/T018 share coverage/effect integration and should merge sequentially. T024 must wait for proven single-worker stage behavior and R003. T025 follows T024. T028 ABI work waits for public Rust API stabilization.

Serialize: `taiyou-types`, contracts/ADRs, native dependency recipes, lockfile updates, capability policies, reference promotion and render coordinator integration. Do not delegate one shared enum to several agents. Integration itself is a named task with a clear owner and failure triage, not “everyone fixes the build”.

## D3. GitHub Actions design

Planned workflows (T001 creates documentation checks and the minimal source lane; later tasks expand these):

| Workflow | Trigger / cost control | Required outputs |
|---|---|---|
| `blueprint.yml` | Every PR, cheap; no Rust setup for docs-only changes | link/schema/task-DAG checks |
| `fast.yml` | Source/manifest changes; one Linux toolchain | fmt, clippy, unit tests, safe API checks; fail fast |
| `focused.yml` | After fast succeeds; trusted changed-module map | relevant stage/golden/determinism results |
| `compatibility.yml` | Every relevant PR and every manual dispatch, no task-branch restriction | full pinned corpus/all status counts, supplemental focused smoke, separate native libass oracle; raw failures and artifacts retained |
| `platforms.yml` | Native/FFI/build changes, milestones and release | Windows MSVC, Linux, macOS ARM64; additional x86/ARM coverage per release support |
| `safety.yml` | Nightly/dispatch, changed native surface | fuzz smoke, sanitizer and pure-Rust Miri lanes |
| `bench.yml` | Maintainer dispatch on controlled runner | raw before/after results; never ordinary hosted timing gate |

Use job-level conditions and an always-running gate aggregator so documentation-only PRs do not leave required checks pending. Path changes to selectors/workflows trigger policy review; they cannot disable their own gate. Rust source changes in types/font/evaluate/raster/compose or native versions select all dependent cases; unknown classification selects the broader suite. Schedule/full validation catches missing coverage mappings.

Cache dependency downloads and native build artifacts by OS/arch/compiler version/flags, Cargo.lock and native source hashes. Cache Rust build artifacts only with compatible keys; untrusted candidate jobs cannot publish cache entries consumed by privileged validation. Cancel superseded PR jobs, cap matrices, use concurrency groups and upload only bounded failure artifacts plus compact summaries. Source compilation happens once per target where possible; focused comparison reuses that binary. Expensive cross-platform full corpora wait for milestone gates.

The CoreV1 full external comparison on relevant PRs is mandatory; it cannot be
deferred to a milestone or replaced with focused smoke to reduce CI cost. The
cross-platform milestone sentence above concerns additional platform-wide corpus
runs, not the ordinary full comparison lane. A failed smoke does not skip the
full run after a successful build. A failing full comparator remains a failed
step/job even when report-integrity checks pass; artifact upload runs after
failure. Manual dispatch has no focused-only mode. See
[full coverage policy](TESTING.md#v5-full-visibility-and-focused-runs) and the
[mandatory agent rule](../AGENTS.md#protected-comparison-tests--never-modify-to-make-ci-pass).

GitHub permissions default read-only, actions pinned by full commit SHA, no secrets for candidate code, no untrusted forks on persistent benchmark runners. Protected runner/harness design is [V4](TESTING.md#v4-protected-validation-trust-model). Git and the GitHub remote are now established; workflow execution and repository/organization protections must still be verified separately before claiming they are active.

## D4. Acceptance and cost control

An implementation task normally fits one focused session. If its preconditions expose an unresolved rule, stop that slice and open a bounded research item; do not spend days guessing. A coordinator may split a task while preserving its ID as an umbrella and declaring child scopes. Use lightweight local document checks followed by focused Actions jobs for source changes; no local compilation. Full matrices are not necessary for prose changes. Keep artifact retention short (initial policy: 7 days for ordinary diagnostic artifacts), upload bounded failure details, and cache dependencies on runners rather than this PC. Do not buy speed by omitting meaningful verification or hiding failures.

Architect review is required for new native/unsafe boundaries, accepted external differences, budget/rounding changes, cache dependency reductions and output reordering. Routine private refactoring within a task is permitted only if necessary for its objective. Every completion report follows [guidelines](GUIDELINES.md#required-report).
