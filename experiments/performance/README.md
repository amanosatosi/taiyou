# Performance phase

Base: merged PR #35, `2a49361a02e0e8f0ace17fa0a45925069eee3201`.
Branch: `perf/cache-multithread`. Work proceeds sequentially: baseline, measured
bounded caches, cached single-worker checkpoint, parallel work, combined matrix.
Existing external comparisons remain read-only. Intentional rendering differences
and unresolved compatibility cases are not new performance objectives.

The Actions-only `performance` example uses the existing rendering path with
single-worker/no-cache defaults. Parsing and font admission occur before the
render measurements. Cold means a fresh renderer, with the same already-admitted
immutable track/catalog. Same-time repeats the warm-up timestamp. Warm-repeat
uses another fixed timestamp; sequence advances by 16,667 us and seek jumps both
forward and backward. Each scenario has 31 samples. p95/p99 use nearest rank;
31 samples do not support strong tail-latency claims. No external renderer ratios.

Opt-in renderer-owned instrumentation measures evaluation, shaping, layout,
collision footprint/solver, geometry, clips, masks (raster/stroke/blur together),
paint planning and composition. Stage durations exclude coordinator overhead and
are successful-call observations; total includes retries. Instrumentation cannot
influence rendering or scheduling. Profiling-on/off bytes and diagnostics are
checked before sampling. Refine a dominant aggregate before selecting a cache.

1080p/4K workloads cover dialogue, simultaneous events, vectors, thick borders,
blur, karaoke, transforms, clips, paint animation, movement and a deliberately
combined heavy case. Multiple borders are not implemented: thick overlapping
border events are a workload proxy. No Mangetsu tags are ported. The experiment
explicitly uses a 512 MiB transient allowance and 32 million span/path work units;
production defaults are unchanged. Separate production-default preflights record
resource rejections in `failures.tsv`. Measurement-configuration failures also
remain visible and make the measurement job fail; available samples still upload.
Output bytes are recorded per frame; process peak RSS comes from `/usr/bin/time`
and includes setup and all workloads, not per-stage retained memory.

Artifacts record raw samples, distributions, commit, CPU, runner image, platform,
compiler and process memory. Hosted runners provide performance smoke only;
controlled hardware remains necessary for definitive claims and automatic worker
thresholds. There is no timing gate. Initial numbers must be recorded and committed
before any caching/threading behavior changes.
