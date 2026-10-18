# Testing and protected validation

The integrated T001–T014 stages have workspace, native and first-render Actions evidence. T005 inventories the separate pinned external corpus; full external comparisons remain T015 work. Reference pins exist in [references.lock.json](references.lock.json). Solo-maintainer governance follows ADR-011; candidate CI and actual GitHub protection are reported separately.

Execution location: all compiler/Cargo/native build commands and renderer, safety and performance test executables run in GitHub Actions under the [CI-first policy](DEVELOPMENT.md#d0-execution-location-and-local-storage). The user's PC is limited to editing and lightweight documentation/hash checks. Preserve every acceptance criterion; moving execution to CI does not waive tests. Reports must identify the candidate SHA and Actions run/jobs. Controlled benchmark hardware is provisioned separately, never assumed to be this PC. CI provisions its own verified reference sources; local Windows archive paths are not runner inputs.

## V1. Test layers

| Layer | Inputs and oracle | Required checks |
|---|---|---|
| Units | Synthetic values, explicit contract results | Parsing/time/color, bounds, blend arithmetic, clipping, layout metrics, cache keys |
| Stage integration | Small pinned fonts and serialized stage dumps | Source spans→clusters→glyphs→baselines→paths→masks→pixels |
| Taiyou fixtures | Independently authored ASS plus manifest | Core exact output; later approved Legacy output; clear scope and provenance |
| libass corpus | Locked upstream assets untouched | Full inventory, native oracle and Taiyou adapter results reported separately |
| xy/original captures | Versioned source/build/environment plus raw captures | Independent profile comparison; original absent remains visible |
| Safety | Malformed corpus, fuzz/property tests, injected limits | No panic/UB/OOB/unbounded loop; deterministic bounded errors |
| Concurrency | Repeated frames, seeks and cache states | Byte equality across worker counts and schedule perturbation |
| Performance | Controlled hardware/build/corpus | Cold/warm timings, work and memory counters; no accuracy tradeoffs |

## V2. Immutable assets and manifest

T005 creates a complete manifest by enumerating the pinned libass-tests tree. Each record includes stable case ID, repository commit, source/font/expected-image SHA-256, timestamp and viewport semantics, scale options, profile, expected support, feature tags, oracle build/config, comparison policy and license/provenance. A case with multiple frames has one record per frame and a stable parent ID. Symlinked fonts resolve only inside the corpus root; record resolved file hashes and preserve mapping on Windows. External assets live in a read-only job directory outside the candidate checkout; no implementation job writes it.

Do not invent SHA-256 values. T005's frozen inventory records 187 external assets and 101 frames at the locked libass-tests revision; it remains separate from Taiyou. Use [local reference sources](LOCAL_REFERENCES.md) first and verify archive identity. CI acquires/caches the exact missing external corpus and verifies all identities before and after execution. Historical support reason strings remain frozen until explicit maintainer promotion; T014's existence does not certify external cases. Keep upstream assets separate from Taiyou fixtures. Unknown redistribution rights do not justify bundling fonts or pretending substitutes are identical.

T015 adapts upstream compare metadata by inspecting its actual naming/timestamp/scale rules. Do not run upstream `run.sh` against a Taiyou CLI and assume compatible arguments. First reproduce upstream native results at the pinned build. Keep upstream reported comparison classifications as context, and publish Taiyou's own raw numeric metrics. Original VSFilter captures require an identified source/binary, Windows version, locale, fonts, frame/sample aspect, input/output pixel formats, color matrices and capture procedure. Reference alpha may require controlled black/white background captures; R001 must validate reconstruction before adopting it. Never compare screenshots with unknown scaling/color conversion.

## V3. Image comparison

Store/display expected, actual, absolute-difference heatmap, changed-pixel bounding box, max channel delta, per-channel MAE, mismatch count and alpha mismatch count. Compare same dimensions and explicitly normalized format. For external straight PNG references, normalize both sides to documented premultiplied encoded RGBA for comparisons; preserve originals. Do not resize, crop, gamma-correct or color-convert silently.

Default for Taiyou arithmetic/determinism fixtures: exact byte equality. External accuracy is per-case/profile: initially **zero tolerance until approved otherwise**, with unsupported cases still rendered/reported where possible. A tolerance proposal must list maximum channel delta, maximum changed pixels and alpha restrictions, with visual evidence. Do not use SSIM alone to hide misplaced/absent glyphs. No corpus-wide blanket threshold to achieve a green dashboard. Different coverage backends may need explicit approved differences; this is a compatibility decision, not a test-runner default.

## V4. Protected validation trust model

[ADR-011](decisions/011-solo-maintainer-validation.md) records the maintainer's
explicit 2026-10-18 decision to use solo-maintainer governance. This supersedes
the original mandatory second-reviewer/separate-validator/App deployment model.
It changes governance, not regression accuracy requirements or C1–C7.

The sole human maintainer reviews and accepts reference revisions, source/font/
image hashes, tolerances, support promotions/demotions, selectors and validation
infrastructure. Protected-by-policy surfaces include validation/**, reference
locks/assets, support/tolerance registries, workflows/selectors, CODEOWNERS and
normative architecture/validation documents. AI agents may prepare proposals but
must not acknowledge their own baseline changes, self-accept or self-merge them.

Ordinary PRs use inventory, policy and verification tools from the exact PR base
revision, not candidate replacements. The corpus job runs on every PR without a
path filter. A base-revision guard detects protected-file changes; the human
owner must post its exact head/base/change-digest acknowledgment and rerun the
job. This acknowledges a proposal, not a new comparison baseline or final task
acceptance. A new commit/base requires a fresh acknowledgment. Candidate policy
changes remain inactive until the maintainer deliberately accepts and merges.
See [setup and promotion process](../validation/MAINTAINER-SETUP.md).

Bootstrap PR #11 uses the immutable owner-reviewed inventory/policy snapshot
39f6ce2fa64b231fd705862d853c8df239b54a22 because main does not yet contain T005.
Its inventory/reference lock and all regression-bearing policy fields must stay
unchanged. The owner-authorized ADR-011 governance/infrastructure changes are the
only bootstrap exception; final candidate acceptance is still pending.

Never regenerate references in ordinary CI. Forbid ART_REG_* overrides and
candidate selectors. Verify every cached corpus against its pin and complete
manifest before/after execution; retain unsupported/failing cases and reject
missing/duplicate/unselected/reclassified results. Normal checks use read-only
tokens, pinned Actions, no repository secrets and no privileged
pull_request_target execution of candidate code. Mark corpus files read-only.
T015 must bound renderer execution and collect actual outputs/metrics rather
than accept a candidate's claimed PASS JSON. No T015 comparator is implied here.

Report these states separately:

- **Inventory verified:** pinned external bytes and complete manifest match.
- **Candidate CI passed:** named ordinary Actions checks passed on an exact SHA.
- **Solo-maintainer accepted:** the human explicitly accepted that exact candidate.
- **Independently trusted:** requires actually independent policy/check authority;
  it is not claimed in this project configuration.

Recommend main protection requiring PRs and passing Corpus policy gate, Fast gate,
Platform gate and Documentation structure, with zero required second-person
approvals, no force pushes/deletions and checks applying to administrators where
available. Record effective settings, not just configuration files. If setup is
pending, identify it; never mark the protection gate operational without evidence.

This is an accidental-change safeguard, not an adversarial trust boundary. A
repository admin can deliberately change baselines, controls or acknowledgments;
an agent using the same owner identity cannot be distinguished from the human by
GitHub. Candidate-controlled CI can be maliciously modified or spoof an Actions
check name. Human inspection of workflow/policy diffs remains essential. Synthetic
tamper tests demonstrate tested checks, not immunity to malicious candidate code.

The original stronger model remains an optional future upgrade: independent
reviewers, a separately protected policy/harness revision, a dedicated required
check issuer, isolated bounded execution with read-only assets and pre/post
hashes, independently computed results and tested issuer/credential isolation.
Only actual deployment evidence justifies that stronger claim. It is not a
prerequisite for T005 solo acceptance or T015.

## V5. Full visibility and focused runs

Every full run reports the entire inventory for each profile with counts for exact/tolerant pass, expected difference, unsupported, regression, crash and infrastructure error. Summary denominators include all cases; supported-subset percentages are separately labeled. Unsupported is a known feature limitation; crashes/timeouts remain failures even on unsupported scripts. Missing fonts cannot masquerade as a feature limitation.

Focused PR reports explicitly say which cases ran, which results are carried from which base commit, and which remain untested; they never label stale results as current passes. Full compatibility runs are dedicated workflows at milestones, manually, and on a cost-controlled schedule once a usable renderer exists. A supported case cannot disappear or become unsupported without protected review. New upstream revisions are separate corpus-upgrade PRs, never floating nightly pulls.

## V6. Determinism and cache tests

Compare complete frames and sorted diagnostics for workers 1,2,4,8, including worker count greater than job count. Repeat 20 times with seeded scheduler perturbation; test cold/warm caches, disabled retention, forced eviction, retained old frames, identical-key bursts, forward/backward/random timestamp orders. Use translucent overlap across tile boundaries, blur halo crossing several tiles, inverse clips, colliding same-layer events, animation and fallback text. Change tile size in test mode (16/32/64/128) and require identical pixels.

Construction counters verify one unique expensive build per key per frame, including cache disabled. Cache-change matrix fixtures mutate one field at a time, compare with a fresh renderer, and check expected reuse. Hash collisions are injected to prove full key comparison. Output memory remains valid after track/catalog/renderer destruction; retained-output budget tests release in different orders. Resource-limit outcomes must not depend on worker count.

Cross-platform runs use identical font files, compiler/dependency recipes and backend settings. Report exact equality separately from approved raster variations. Same-target worker determinism has zero tolerance regardless of external reference tolerance.

## V7. Safety, fonts and malformed input

Parser fuzzing: truncation at every byte, invalid UTF-8, huge numbers/NaN text, duplicate Format/style fields, embedded commas/braces, deep tags, long drawings, bad encodings, font attachment sizes and excessive events. Geometry/effect fuzzing: extreme offsets, degeneracy, winding, perspective near plane, self-intersections, zero-axis borders, large blur and integer overflow. All receive size/operation/time limits.

Use cargo-fuzz/libFuzzer in dedicated Linux jobs; archive minimized seeds and add regression cases. Run Miri on pure Rust unit tests (not unsupported native calls), ASan/UBSan on native adapter/integration builds, and available thread/race instrumentation on parallel lanes. Use injected allocation/limit failures rather than relying on actual host exhaustion. Safety checks do not require matching malformed output to a crash-prone oracle.

Font fixtures cover Latin ligatures/kerning, Arabic joining, mixed RTL/LTR with punctuation, combining marks, Indic reordering, CJK fallback, missing glyphs, spaces/empty lines, repeated same-family fonts, collection face indices, variable axes where supported and malformed fonts. Fonts have explicit hashes/licenses. No assumption that an “Arial” request yields the same font on all systems. Native font parser vulnerabilities remain dependencies to track.

## V8. Performance and KFX

T029 creates corpus manifests for authorized real-world KFX and independently generated stress scripts: thousands of short events, shared glyphs with changing paint, rotation/scale, animated clips, karaoke sweeps, large blur and dense overlap. Do not redistribute copyrighted subtitle/font assets without permission. Record operation counts, render dimensions, frame sampling and supported features so work is comparable.

R003 establishes repeatability on a Ryzen 5 5500U-class controlled runner: fixed power mode, cooling state, OS, CPU topology, compiler flags, native libraries and background load. Warmup, then at least 30 samples per workload/configuration and repeated independent runs. Publish median/p95/p99, CPU time, peak RSS and cache/allocation counters; retain raw samples. Calibrate a regression threshold from observed variance before making timing gating. Hosted Actions timings are smoke signals only. Scaling reports include absolute latency and total CPU work; speedup alone can reward an inefficient baseline.

## V9. Failure artifacts

Name reports by task/commit/profile/case/time/worker count. Include stage responsible, input fingerprint, last completed stage, diagnostic codes, expected/actual image paths and counters. Use opt-in bounded stage dumps for glyphs/layout/paths/masks to localize failures. No output may contain arbitrary font bytes or huge source dumps by default. All failing or unsupported cases link to a feature/issue ID and retain the full compatibility summary.
