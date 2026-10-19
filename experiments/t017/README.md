# T017 review experiments

All executables, Cargo commands and measurements run in GitHub Actions only.
No experiment changes an external comparator, reference or accepted outcome.

- `crates/taiyou-render/tests/previous_collision.rs` preserves the exact reviewed
  solver blob `033ddf4cdee97d34886b1ccac6f68e2e354cb064` from
  `9eee96fe16989089d8fd3f65b1bb136455ffca7e`. It is an independent small-input
  oracle, compiled only as a test module/target, never production. New randomized
  inputs include fractional boxes, layers, directions, starts, empty boxes and
  unrelated explicit positions. Existing regression tests remain untouched.
- `collision_interval_review` records 100/500/1000/5000-event dense solver
  measurements on Linux native Rust1.99.1. It measures the new solver three times
  and records operation counters, conservative workspace reservations, observed
  vector backing capacities and isolated child-process peak RSS. Old
  dense obstacle checks are independently derived as N(N-1)(N+4)/6. Old runs
  above 200 million checks are not executed; their runtime is null, never a
  fabricated measured speedup. Small old runs execute for comparison.
- `measure.py` builds the exact reviewed and candidate Rust CLI in separate
  Actions temporary snapshots, with identical measurement-only atomic counters.
  It reuses the native job's verified FreeType/HarfBuzz prefix, without rebuilding
  native libraries. The source/instrumented hashes, build logs, font pin, OS/CPU/
  compiler, input scripts and raw samples are archived. Each case runs three
  before/after pairs in the same job. `/usr/bin/time` measures process peak RSS;
  counters separately record peak requested transient reservation, peak retained
  local-stage reservation and actual shape calls. Conservative reservation bytes
  are not claimed as actual heap allocation or process RSS. Debug timings are
  hosted-runner observations, not release-performance guarantees.
  Subsequent remeasurement retains every case and all three pairs, with two
  bounded case workers. Each case's pairs remain sequential. Per-process RSS,
  CPU time and wall samples are recorded with this concurrency; wall samples
  must not be compared directly with the original sequential run as speedups.
  The original 5,000-event/four-glyph baseline exceeded the explicit 180-second
  experiment timeout at run39183812909/job117841967333; no completed frame or
  RSS is claimed for it. Its input and the unexecuted 2,000-event/twenty-glyph
  input remain in the unavailable-evidence manifest. Separately named one-glyph
  KFX workloads measure event staging without that unrelated native glyph cost.
- `wrapping_workload_review` measures real candidate shaping for independently
  authored Latin/Arabic/Devanagari/Japanese whole-word/ideograph ranges. It is not
  a production automatic wrapper or Unicode conformance implementation. Search
  stops at the W1 experimental candidate/glyph budgets, or a separately reported
  60-second experimental budget. No fallback or approved Core policy is inferred.

New measurements use the existing native workspace test lane and existing
`TAIYOU_RENDER_SAMPLE_DIR` artifact upload. No workflow, required check, corpus
selector or harness is modified. The experiment tests are additive. Numeric/
randomized correctness tests run normally; expensive measurements run only on
the Linux 1.99.1 native recipe, avoiding repeated platform/MSRV benchmarking.

The PR records actual runs/results/artifacts and any measurement failures.
An unexecuted or incomplete sample remains explicitly unavailable. The full
external comparator's genuine nonzero result and artifacts remain separate.

## Adaptive follow-up measurements

The owner authorized workspace snapshot isolation: remove only workspace-package
artifacts between snapshot builds, preserving the verified native prefix and
registry dependencies. Each build log must show the font crate compiled from
that snapshot. The original stale-artifact and generated-driver failures remain
under `results/6bb3a30/` and `results/6d1e242/`, respectively. Neither produced
successful paired evidence. `results/2574e17-partial/` preserves 99 actual release
samples and the first genuine default frame-segment failure; it is incomplete.

`optimized.py` measures the earlier implementation, reviewed `027e58a` and the
candidate with the same release profile, pinned font, native prefix and runner.
It records three sequential samples per configuration: cold CLI wall/CPU time,
peak process RSS, conservative reservation peaks, stage calls/timers, original
scripts and raw RGBA hashes. These are cold-process measurements including font
intake and PNG output, not warm playback FPS. Inclusive stage timers overlap;
shadow timing covers the whole visible-shadow branch. Full/partial/minimal
staging configurations use explicit caller budgets. The complete suite includes
HD typesetting in addition to deliberately small staging stress scenes.

The 500-event multiglyph scenes retain their genuine default-limit failures.
Separate, identically configured 4,000,000-segment caller-budget samples determine
whether these workloads complete with sufficient resources. This is not a
production-default change. Failed calls keep their exit codes and error details,
have no frame/hash, and never count as matching rendered outputs. Any newly
failing candidate with a successful reviewed configuration fails the experiment.

The width-only experiment is independently checked against full line shaping on
all native platforms; timing runs only in the Linux lane. It shares original
source ranges and actual boundary shaping, skips rejected-candidate ink extents,
and records preparation, shaping and logical-extent costs separately. Polynomial
search tests compare a reusable candidate DAG with exhaustive small nonmonotonic
plans. This neither enables wrapping nor proves Unicode conformance.

The owner also authorized evidence-preserving reuse in `reuse_completed.py`.
Only complete, individually hashed original reports listed in `results/completed.json`
can be reused after Git checks measured production/build/input code is unchanged.
Complementary experiment-script changes do not invalidate the other experiment.
Deleted tests and changes to preexisting tests are rejected. Owner-authorized
append-only additions to experiment tests introduced after the reviewed baseline
are admitted only when the complete measured file remains an unchanged byte
prefix; original functions/assertions cannot be rewritten under that exception.
Every workload and
sample must remain present; invalid, duplicate, failed-as-match or mismatched
frame evidence is rejected. Reuse reports its original SHA/job/environment and
current SHA explicitly. All ordinary correctness gates and the full external
comparison continue to execute on the current candidate.

All 48 smaller paired samples completed at `f473e81` in
[job117845262915](https://github.com/amanosatosi/taiyou/actions/runs/39184889909/job/117845262915).
`results/f473e81/` preserves the original artifact JSON bytes (ZIP SHA-256
`2855e4129608e390f776ff6795273207b09ac520ef995faedd1fba768b8f636b`).
`reuse_or_measure.py` avoids repeating this 18-minute supplemental experiment
when Git proves only the named new review fixtures, documentation and evidence
have changed. Any production/build/input/measurement change runs it again.
Reuse publishes its original candidate/environment and explicit reuse provenance;
it never claims new timings, skips correctness tests or controls the comparator.

The adaptive follow-up's complete48 debug samples are preserved atae467c9.
The [complete release report](results/77afbd2/summary.md) preserves144 attempts,
including18 genuine default-limit failures and18 successful expanded-budget
samples, with original scripts, identities and sanitizer evidence. Both complete
sets are individually hashed in `results/completed.json`; later documentation
checkpoints reuse their original provenance while rerunning all correctness gates.
