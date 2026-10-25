# Initial uncached measurements (partial baseline)

Measured candidate: `e2e9ee6f079b9e9c02f2f77d3f27c2263ce926f5`.
Integration base: `2a49361a02e0e8f0ace17fa0a45925069eee3201` (PR #35 merged).
[Actions run](https://github.com/amanosatosi/taiyou/actions/runs/40236948985),
[baseline job](https://github.com/amanosatosi/taiyou/actions/runs/40236948985/job/121300438474),
[original artifact](https://github.com/amanosatosi/taiyou/actions/runs/40236948985/artifacts/12615332284).

Single worker, cache off, release, rustc 1.99.1 (58cf8f9c3), Ubuntu 24.04.5
hosted image 20261012.334.1, AMD EPYC 7763 VM with four logical CPUs. Same pinned
native recipe and explicit Source Sans 3 font as stage tests. Instrumentation was
enabled. Parsing/font admission are excluded. Experiment transient cap 512 MiB,
default one-million work-unit cap. No controlled-hardware claim.

Only 1080p dialogue finished: 31 samples in each of five playback scenarios.
Raw samples are retained in `initial-1080p.csv`; nearest-rank distributions in
`initial-1080p.json`. Cold median 42.029 ms, repeated timestamp median 42.088 ms.
Masks/strokes/effects dominate (~36.5 ms), followed by composition (~3.5 ms),
geometry (~1.8 ms), shaping (~0.14 ms). No retained caches existed, so repeated
timestamps did not improve rendering. No cache/threading changes precede these
committed measurements.

**The run failed, not passed.** The next 4K dialogue warm-up returned
`LimitExceeded`, event 0, source 217..287, `single-axis span widening budget`.
Later workloads were not reached. Native render unit/stage tests passed before
the benchmark. Peak process RSS was 38,608 KiB across setup and this partial run;
it is not the memory requirement for all workloads. A following baseline revision
records resource failures separately and uses a larger explicitly reported
experiment work allowance to measure 4K/heavy cases without production changes.
