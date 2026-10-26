# Fixed-sample cached multithreading checkpoint

Measured renderer `dccd0fded86fbc24acd1ebbfaedafa8e5234db6c`;
[green correctness/scaling run](https://github.com/amanosatosi/taiyou/actions/runs/40303676779),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40303676779/job/121507413141),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40303676779/artifacts/12643162994).
Intel Xeon6973P-C VM, four logical CPUs. Source/configuration/renderer changes
stop here; subsequent commits admit the exact dependencies and record evidence.
No new optimization is selected. Default caching is bounded under the existing
128MiB cache limit; default workers stay1, with no automatic scheduling threshold.

All six workloads, both1080p/4K, four scenarios and31 fixed samples execute in
OFF/1 and ON/1/2/4/8 modes. Thus1488 samples/mode and7440 sampled frames. The strict
SHA-256 audit reports zero RGBA or diagnostic differences, with hashing outside
the timer. Existing supplemental native tests directly compare actual bytes over
clips, blur, karaoke, transforms, seeks, fonts/documents, retained frames and tiny
budgets. Native stages pass. Actions formatting patch is empty. No local builds.

Warm repeated medians in ms (all31 samples; p95/p99 in summary.json):

| Workload | 1 | 2 | 4 | 8 |
|---|---:|---:|---:|---:|
| Dialogue1080p | 5.28 | 5.19 | 5.37 | 5.55 |
| Thick borders1080p | 8.01 | 8.89 | 7.57 | 7.73 |
| Dialogue4K | 21.95 | 22.06 | 22.29 | 19.09 |
| Large blur4K | 25.62 | 23.42 | 28.17 | 25.81 |
| Karaoke4K | 16.60 | 17.99 | 21.73 | 21.74 |
| Combined4K | 128.23 | 128.97 | 91.52 | 83.00 |

Combined4K by playback state:

| State | 1 | 2 | 4 | 8 |
|---|---:|---:|---:|---:|
| Cold render | 425.93 | 421.64 | 381.12 | 383.25 |
| Warm same timestamp | 128.23 | 128.97 | 91.52 | 83.00 |
| Changing sequential geometry | 432.35 | 431.09 | 382.90 | 385.14 |
| Repeated seeks | 163.41 | 126.73 | 103.75 | 103.70 |

Four workers is the explicit balanced MT configuration for the next same-runner
libass comparison: it helps the expensive workload without oversubscribing this
VM. Eight wins the warm stress median but not changing/cold throughput; light
workloads often slow down. This does not justify changing the default from one
or inventing an automatic threshold. Hosted timings remain noisy observations,
not controlled-machine guarantees or hard regression gates.

Warm combined4K hits100%, retained2330.11KiB. Changing geometry hits~93%, with
54,742,360 bytes retained at peak across31 frames, below the fixed128MiB cap and
128-entry bound. Cold still reuses equal within-frame requests. Composition falls
113.64→75.39ms at4 workers; changing frames still spend~208ms in serial span union.
Warm FPS equivalents are~7.8 at1 worker and~10.9 at4; sequential equivalents are
~2.3/~2.6. Full measured distributions, means/FPS, counters and process RSS/CPU
include their actual counts in the adjacent evidence. Font/source/pool setup is
separate; render timing excludes hashing/CSV writing. Production-default resource
rejections remain separate in all failure TSVs; no production cap increased.

The [full default-cache-on external comparison](https://github.com/amanosatosi/taiyou/actions/runs/40303684818)
preserves all101 recorded Taiyou raw digests (zero changed outputs). Its separate
libass oracle remains red with1 exact visual frame,100 pixel mismatches,0 visual
unsupported/crash/infrastructure cases;2 parser probes and1 upstream-disabled
source are separate. No tolerance/reference/classification changed. Full platform
and Fast gates require the exact dependency-intake follow-up; these failures are
recorded rather than being described as passed. The maintainer authorized only
the documented T017 lock hash pair and negative probes; the current measurements
are not reused historical samples.
