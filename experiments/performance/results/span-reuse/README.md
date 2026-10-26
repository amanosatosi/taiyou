# Duplicate-row reduction: measured single-worker checkpoint

Candidate `eedd7d3d81860b4ff5df8a651c8e5b13dedec9a5`;
[green focused run](https://github.com/amanosatosi/taiyou/actions/runs/40245285817),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40245285817/job/121325435680),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40245285817/artifacts/12618662103).
EPYC 9V45 VM, four logical CPUs. Native build/stages, added native-cell union
tests, resource-witness tests and existing supplemental exact cache tests pass.
No compiler or test executable ran locally. Prior failed run `40245103674` stopped
on invalid native-directory cache reuse before renderer checks; it supplies no
performance evidence. Cargo-only reuse fixes that optional workflow failure.

Actual generated candidates / original producer candidates: dialogue ~45–47%,
thick borders ~44%, combined stress ~68–69%. This is a structural count, independent
of runner speed. Identical occupied native-grid cells and unchanged frame bytes
are required; no tolerance or production cap changed.

Same-runner/current-binary repeated timestamp medians (ms):

| Workload | No cache | Bounded masks + filters | Samples off/on |
|---|---:|---:|---:|
| Dialogue 1080p | 17.25 | 3.55 | 31 / 31 |
| Dialogue 4K | 54.51 | 11.21 | 31 / 31 |
| Thick borders 1080p | 204.38 | 5.93 | 10 / 31 |
| Thick borders 4K | 865.32 | 17.87 | 3 / 31 |
| Combined 1080p | 869.64 | 32.94 | 3 / 31 |
| Combined 4K | 3607.11 | 110.42 | 3 / 18 |

These are not the original Stage A numbers: both modes include span reduction.
Earlier committed pre-optimization samples/failures stay intact. Different EPYC
runner models preclude attributing every difference across runs to this change.
Raw distributions and sampling counts are in the adjacent files; short hosted
samples do not establish tail-latency or controlled-hardware performance claims.

All selected benchmark scenarios now complete, including thick-border 4K that
previously exceeded the experimental 512 MiB/32M envelope. Production-default
thick-border, large-blur and combined 4K preflights still reject the configured
work cap and remain in both failure reports. Nothing silently raises normal caps.

Combined 4K cache warm hits 100%, retention 2330.11 KiB. Warm composition remains
96.32 ms / 110.42 ms total. Changing geometry sequence is 328.53 ms, union159.19 ms,
composition97.83 ms, filters29.84 ms; aggregate hit rate94%. Seek median164.09 ms,
hit rate97%. These time-capped groups do not use equal timestamp extents across
modes, so no sequence-throughput speedup is asserted here. Selected peak retained
cache ~16 MiB; process peak RSS off173496/on170724 KiB includes all setup/workloads.

Parallel composition is justified by the cached warm stage cost; raster/filter
misses remain measured serial bottlenecks, rather than being hidden by a frame cache.

The [full external comparison](https://github.com/amanosatosi/taiyou/actions/runs/40245289121)
preserves all 101 actual Taiyou raw-output digests: zero changed frames versus the
frozen cached single-worker observation. `frame-audit.json` records this separate
audit. The independent libass suite remains red: one exact visual match, 100 pixel
mismatches, no unsupported/crash/infrastructure frames. Parser probes remain
separate. The existing comparison algorithm and acceptance policy are untouched.
