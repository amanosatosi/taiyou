# Second cache layer and cached single-worker evidence

Candidate `9fc4b5719682a5e278c85a059ef56e2192242a26`;
[green correctness/measurement run](https://github.com/amanosatosi/taiyou/actions/runs/40242290399),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40242290399/job/121316470399),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40242290399/artifacts/12616894414).
EPYC 9V74 VM, four logical CPUs, same binary/runner for mask-only and full-cache
processes. Hosted smoke only. Original pre-optimization baseline stays intact.

Repeated timestamp medians (ms), with actual counts/distributions in CSV/JSON:

| Workload | Masks only | Masks + filters | Filter stage after | Retained KiB |
|---|---:|---:|---:|---:|
| Dialogue 1080p | 6.62 | 6.28 | ~0 | 104.06 |
| Dialogue 4K | 17.28 | 17.16 | ~0 | 219.04 |
| Large blur 1080p | 29.32 | 8.03 | 0.08 | 183.06 |
| Large blur 4K | 94.47 | 25.89 | 0.36 | 483.23 |
| Combined 1080p | 254.90 | 58.41 | 1.18 | 649.01 |
| Combined 4K | 934.60 | 186.97 | 4.58 | 2330.11 |

Full-cache warm hit rate is 100% for these cases. Mask-only mode records filter
producers as misses, so its 60% aggregate hit rate on blur fixtures is not the
first layer's raster-only statistic. Combined 4K cold is 691.93 ms, sequential
changing geometry 695.22 ms, and seek 829.86 ms (time-capped groups, not equal-length
sequence speedup claims). Their cache footprints stay bounded; worst observed
selected seek retention is ~13 MiB. No hard performance gate is introduced.

Native unit/stage tests, all four mask/filter cache unit tests, and all three
supplemental exact-frame/lifetime/eviction tests pass. Filter tests vary complete
input bytes, units, positions and parameters and retain low-memory failure checks.
1080p/4K frame fixtures require exact successful RGBA and diagnostics in both modes.
Cross-platform exact-head validation remains pending formatting application; this
is the tested Linux cached single-worker reference, not a merge acceptance claim.

Combined 4K warm composition is 163.12 ms out of 186.97 ms; changing-frame span
union is still 385.90 ms, expansion 64.98 ms. A narrow duplicate-span work reduction
is justified before parallel execution. No production budget is increased. The
full prior thick-border failure reports remain preserved; this layer's focused
selection excludes that known over-budget benchmark, not any correctness test.

Process peak RSS: masks-only 207444 KiB, full cache 213680 KiB. This includes setup
and all selected work; separate retained counters are more specific to the cache.
The independent frozen-libass full comparison remains strict/red: 1 exact, 100
pixel mismatches, 0 unsupported/crashes/infrastructure errors; two parser probes
are separate. No reference/tolerance/accepted outcome was changed or weakened.
