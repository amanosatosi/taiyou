# Recovered parallel-composition smoke evidence

Candidate `af0dee79dc5215aac81eaf94d6c30c6884d4afff`;
[green run](https://github.com/amanosatosi/taiyou/actions/runs/40246064679),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40246064679/job/121327765062),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40246064679/artifacts/12619395339).
Intel Xeon Platinum8573C VM with four logical CPUs. Existing stage tests and the
new actual-byte cache/worker matrix pass, including seeks, font/document changes,
retained frames, resource failures, selected 4K fixtures and seeded band tests.
The workflow generated the pinned Rayon resolution and formatting patch on CI.
Adjacent Cargo.lock/dependencies.json preserve that resolution; the root lock
also adds the already-resolved SHA-256 direct edge for subsequent frame audits.

Warm repeated medians (ms), same binary and runner:

| Workload | 1 worker | 2 | 4 | 8 |
|---|---:|---:|---:|---:|
| Dialogue1080p | 5.36 | 5.45 | 5.52 | 5.63 |
| Dialogue4K | 14.27 | 14.41 | 15.58 | 15.61 |
| Vector4K | 37.13 | 37.41 | 27.57 | 27.00 |
| Combined4K | 140.51 | 138.00 | 89.35 | 90.54 |

Combined4K composition falls119.26→68.32 ms at4 workers. This is real parallel
pixel work; two workers offer little benefit for these row distributions and
eight oversubscribe this four-CPU VM without improving combined-frame time.
Light content is slower with extra workers. Default remains one;4 workers is an
explicit useful heavy-work comparison setting, not an automatic global policy.
No threshold, new scheduler or additional optimization is selected.

The run used time-capped sampling (combined4K counts15/15/22/22). It is valid smoke
evidence, but not the final equal-length animation comparison. A focused fixed31
sample/timestamp run will supply the frozen checkpoint and per-sample RGBA and
diagnostic SHA-256 audit outside render timing. Raw samples/distributions and
process RSS/CPU evidence remain adjacent. Production-default resource rejections
are separate from successful expanded-allowance benchmark frames.
