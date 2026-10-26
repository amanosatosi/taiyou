# First serial optimization: validated contiguous mask/RGBA rows

Candidate8f44e6518a4b4ab8d03d6277163dcaa8ff7f40af, reference production checkpoint
2b80565e4998e22032d6bb4a446013886df7c62c;
[focused run](https://github.com/amanosatosi/taiyou/actions/runs/40319452195),
job121560335436, artifact12648248852. AMD EPYC9V74 VM, four logical CPUs.
Only the serial pixel traversal changes. Same integer blend kernels, operation
order, complex/inverse clip fallback, cache, resource checks and worker policy.
No unsafe code or dependencies. Default workers remain1; no new parallel stage.

All2604 common-input samples complete, zero process failures, zero candidate vs
same-runner frozen1T RGBA/diagnostic differences, zero changes vs the original
Taiyou benchmark digests. Focused native renderer tests and supplemental cache,
worker, seek, clip, font/document, retained-frame and resource tests pass. The
independent coordinate-sampler test includes both coverage units, padded masks,
negative/huge offsets, rectangle/inverse/mask/intersection clips and mixed alpha.
CI-generated formatting is retained in format.patch and applied locally afterward.
Fast on the unformatted input was red for formatting; no failed check is hidden.
The explicitly authorized draft probe leaves full platform/comparison gates red.
This is measured candidate evidence, not a completed full-validation checkpoint.

Paired medians in ms (31 samples/group; all distributions/raw rows adjacent):

| Workload/state | Before1T | Row1T | libass RGBA | Row/libass |
|---|---:|---:|---:|---:|
| Dialogue1080p warm |5.316|4.964|0.722|6.88x|
| Borders1080p warm |9.005|7.111|2.459|2.89x|
| Dialogue4K warm |14.258|12.955|3.216|4.03x|
| Stress4K warm |157.377|60.092|80.058|0.75x|
| Stress4K changing |452.143|350.320|86.492|4.05x|
| Blur4K warm |20.008|15.281|8.030|1.90x|
| Transform4K changing |65.158|58.482|10.794|5.42x|
| Upstream karaoke1080p warm |4.991|4.299|1.033|4.16x|

Warm stress61.8% cheaper, changing stress22.5% cheaper. The initial row change is
worth retaining, but remaining serial gaps are substantial: prioritize changing
span union/raster work and measure full-frame publication separately. Neither a
good warm nor MT number satisfies the serial objective. Different CPU from the
controlled input baseline: use THIS paired reference for improvement claims.
Painted-area evidence remains unchanged; libass normal images/warnings are
recorded in samples.csv. Cache-off and resource behavior are strict native tests.
The101 historical frame digest audit remains required on a kept full checkpoint.
