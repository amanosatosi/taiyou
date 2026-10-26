# Raster architecture baseline: generation / bucket / sort / union / writes

Candidatee56aef54a4fb42cee17d474b2ee65a6ff5bf6f29; reference9265bb3b90b7c17ce07cbf84d60b88ae40ec578f.
[Green focused run40346334826](https://github.com/amanosatosi/taiyou/actions/runs/40346334826),
job121654122733, artifact12661847212. No raster algorithm changes in this run.
4340 primary frames; zero process errors, candidate/reference RGBA/diagnostic,
original Taiyou digest or normal cache OFF/ON resource differences. Additional
31-frame profiles for three4K sequences compare exact hashes outside timing.
CI formatting is applied after recovery. Full gates remain deferred/unsatisfied.

Primary stress4K changing medians: Taiyou374.172ms, paired reference374.108ms,
stock libass direct2.005ms, Mangetsu legacy2.958ms, Mangetsu RGBA24.255ms. The
timer-disabled instrumentation control is effectively unchanged on this runner.
Exact compiler/CPU/build/source/font provenance and distributions are adjacent.

The additional changing profile uses further16ms steps after the primary sequence,
not identical timestamps to that primary median. Median ms across31 observations:

| Observed work | Median ms |
|---|---:|
| Full profiled render |401.728|
| Segment/crossing generation |0.974|
| Initial edge sort |3.058|
| Base span generation |0.338|
| Border expansion including its source-band preparation |42.064|
| Expanded-span row bucketing |110.142|
| Interval sorting including source-band sorting |111.618|
| Interval merging |7.292|
| Explicit raster Vec reserve/reallocation |3.476|
| Bitmap clearing |0.055|
| Fused coverage arithmetic + destination accumulation |3.110|
| Bitmap publication/validation |0.497|

Allocation overlaps its owning stage; source-band sorting is also inside border
expansion. Do not sum these columns as disjoint costs. Coverage arithmetic and
writes are fused, so the profile reports the actual combined loop, not invented
exclusive store timings. Allocation timers run only at capacity growth. The first
observation records194 reserve operations (164 reallocations),131,271,350 cumulative
capacity-growth bytes,4,280,934 expanded candidates and2,207,948 coverage writes.
These counts vary with geometry; raw lines retain every observation.

The measured problem is global movement/sorting of expanded intervals. Neither
allocator tuning nor SIMD bitmap writes is the first target. See raster-boundary.md
for pinned-source verification and the smallest exact closed-fill tiled slice.
Keep existing Taiyou semantics and legacy resource witnesses; do not substitute
libass analytic A8 coverage or outline-stroke behavior for native xy coverage.
