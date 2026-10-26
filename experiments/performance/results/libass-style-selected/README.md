# Controlled shared styles and explicit event style selection

Commit 27a2e481f90516bad69e25bfe3b6a92a757df4ca;
[green run](https://github.com/amanosatosi/taiyou/actions/runs/40318434501),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40318434501/job/121556844327),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40318434501/artifacts/12648356151).
Intel Xeon 6973P-C VM, four logical CPUs. All 2604 frames execute, 31 samples per
group, one warmup, same files/fonts/dimensions/integer-ms timestamps for all engines.
Zero process failures; zero Taiyou1/4 RGBA or diagnostic differences; zero Taiyou
RGBA changes versus the retained original sparse-input run. Hashing, diagnostics
and painted-area scans are outside the primary render timer. Neither renderer
changed. This is the final controlled baseline before serial optimization.

Explicit style values AND event Style=Default resolve the accidental smaller
libass font/effect workload. Warm dialogue painted area is now12849/12774 rather
than12849/3124. All libass warning counts are zero. Retained sparse and style-only
runs remain evidence, with their workload-control caveats; they are not replaced.
Residual area differences (notably blur/stress) reflect differing semantics and
remain visible, never grounds for rejecting performance samples or changing pixels.

libass0.17.5, API0x01705000, commit f61db567e6593df3470e91594bcd4ad2d0473aff;
normal renderer, caches, complex shaping and CPU dispatch, no internal timers.
Same pinned FreeType2.14.3/HarfBuzz14.5.0 as Taiyou, distro FriBidi1.0.13. Exact
source/font hashes, compiler/build flags, platform and all samples are adjacent.
Full per-process memory/CPU/stderr reports remain in the artifact. Primary libass
time includes fresh owned RGBA allocation, clearing and scalar composition on
every frame, even when its image list reports unchanged. No finished-frame reuse.

See 1t-table.md for the primary serial comparison and image/painted-area evidence.
table.md preserves the requested frozen MT comparison without using its speed
as the serial optimization target. Cold rows include renderer creation plus first
render, with shared font/script admission separately reported. JSON/raw samples
include p95/p99, render-only cold, seeks, cache memory and throughput; ratios are
hosted-runner observations, not hard performance gates. Do not compare medians
against earlier runs on different CPUs as optimization improvements.

Prior stage profiling identifies warm composition (113.64ms of128.23ms stress on
the fixed-worker runner) and changing-geometry span sorting/union (~208ms) as the
first serial targets. Those are prior stage observations, not internal libass
timings or falsely precise same-run stage attribution. The next candidate is a
narrow safe row traversal of existing immutable masks with unchanged integer
blend kernels, allocations, resource checks, operation order and worker policy.
