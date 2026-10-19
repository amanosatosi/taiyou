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

All 48 smaller paired samples completed at `f473e81` in
[job117845262915](https://github.com/amanosatosi/taiyou/actions/runs/39184889909/job/117845262915).
`results/f473e81/` preserves the original artifact JSON bytes (ZIP SHA-256
`2855e4129608e390f776ff6795273207b09ac520ef995faedd1fba768b8f636b`).
`reuse_or_measure.py` avoids repeating this 18-minute supplemental experiment
when Git proves only the named new review fixtures, documentation and evidence
have changed. Any production/build/input/measurement change runs it again.
Reuse publishes its original candidate/environment and explicit reuse provenance;
it never claims new timings, skips correctness tests or controls the comparator.
