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
forward and backward. Each scenario has up to 31 samples; hosted smoke stops after
two seconds once at least three samples finish, with actual counts recorded.
p95/p99 use nearest rank; these counts do not support strong tail-latency claims.
No external renderer ratios. Definitive controlled runs still need longer samples.

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

The optional performance workflow now runs on explicitly marked `[perf]` commits
or manual dispatches. Formatting/documentation follow-ups do not rerun expensive
measurements. Existing mandatory correctness, platform and full external comparison
workflows are unchanged. Skipped measurement jobs are never reported as passes.

Completed evidence: [uncached baseline](results/uncached/README.md),
[dominant mask split](results/mask-split/README.md),
[mask retention](results/mask-cache/README.md),
[filter retention and cached checkpoint](results/filter-cache/README.md), and
[duplicate-row reduction](results/span-reuse/README.md). These records remain
immutable observations of their named candidates, including resource failures.

Final matrix runs use fixed sample counts at identical timestamp sequences. They
record workers and band rows, separate font/source/renderer setup costs from
render time, and hash RGBA plus diagnostics after stopping the render timer.
The supplemental digest gate requires identical sample inventories and digests
in OFF/1 and ON/1/2/4/8 modes. Separate native tests
compare the actual RGBA bytes directly. SHA-256 verification and CSV writing
overhead are excluded from render-only FPS equivalents; process CPU/RSS metrics
include setup and audit overhead. Cold resets both caches and the renderer-owned
pool; pool creation is recorded separately. No local benchmark execution.
