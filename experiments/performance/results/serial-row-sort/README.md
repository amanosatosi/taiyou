# Second serial optimization: exact scan-row partition before interval sorting

Candidatebc7a38b57c29b7fef54a8d0b40e6cfe8bacd4580; paired reference8f44e6518a4b4ab8d03d6277163dcaa8ff7f40af
(the retained row compositor, NOT the original uncached renderer).
[Green focused run](https://github.com/amanosatosi/taiyou/actions/runs/40322750959),
job121571579785, artifact12649917580. AMD EPYC7763 VM, four logical CPUs.
All2604 primary samples complete, no process errors, no RGBA/diagnostic changes
versus the paired parent and no Taiyou RGBA changes versus the original benchmark.
Fast, formatting, native renderer and focused cache/worker/resource tests pass.
Full required gates remain explicitly deferred/failing for this draft probe.

The earlier8ee8499 run40322400678 stopped at Clippy's unnecessary_map_or after
initial native renderer tests; it has no performance result. Corrected the lint
without changing semantics and applied its CI formatting patch. This successful
run's format.patch is empty; no local compilation or formatting command was run.

The exact same expanded spans are partitioned by integer y with two optional
bounded arrays, then sorted by left/right within each row. Final lexicographic
order and existing interval union/coverage accumulation stay unchanged. Scratch
fits the existing producer peak witness or falls back to the original full sort.
Default caps/cache/resource witnesses, candidate count and required work remain
unchanged. The seeded exact-sort test covers negative/dense rows, duplicate
intervals, extreme/sparse ranges and insufficient scratch; no unsafe/dependency.

Paired median ms,31 samples/group (distributions and all groups adjacent):

| Workload/state | Row1T before | Partition1T | libass RGBA | Partition/libass |
|---|---:|---:|---:|---:|
| Dialogue4K warm |22.398|21.958|4.484|4.90x|
| Stress4K warm |77.057|77.178|93.448|0.83x|
| Stress4K changing |447.059|407.447|101.493|4.01x|
| Transform4K changing |79.274|67.674|13.800|4.90x|
| Borders1080p cold session |63.625|50.983|5.093|10.01x|
| Blur4K cold session |146.967|125.162|13.227|9.46x|

Changing stress8.9% cheaper (~39.6ms); changing transforms14.6% cheaper. Warm
results should be unchanged since the sort is cached out. Keep this measured
reduction, but it is insufficient: span union remains the largest changing cost.
Different CPU from the previous row probe: improvement claims use this paired
parent, not cross-run medians. Image count and painted-area evidence unchanged.

Post-sample observations are outside ALL primary samples. One additional timed
stage frame uses the last timestamp (+16ms for changing sequences);31 independent
publications clone source pixels and reserve output BEFORE timing, then verify
published bytes afterward. They are diagnostic observations, not extra primary
frames or libass internal instrumentation. Process CPU/RSS includes these probes.
Selected warm/sequence Taiyou stderr text is retained; complete process reports
remain remote in the artifact. Examples:

- Dialogue4K warm total21.79ms, composition19.57ms; isolated publication median13.91ms.
- Stress4K next changing frame: span union275.61->242.08ms; border generation44.97->45.74ms,
  filtering59.29->56.39ms, composition55.99->54.39ms. Same4,256,926 candidate spans,
  248,414,936-byte resource peak and3,443,207 required-work witness.
- Transforms4K next changing frame: span union35.95->24.37ms. Publication~7.1ms.

The next bounded serial investigation targets the demonstrated public frame
publication cost, preserving premultiplied-pixel validation, all capacity/length/
reservation checks and retained-frame leases. Further span reduction remains
necessary; a good warm stress number does not satisfy the single-thread goal.
