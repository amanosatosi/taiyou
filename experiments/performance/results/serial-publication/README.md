# Publication experiment: rejected as-is after mixed measured results

Candidate67f6b7f1041d0755a83ae72545e3eda0e0eb87a8; paired retained reference
bc7a38b57c29b7fef54a8d0b40e6cfe8bacd4580.
[Green correctness/performance experiment](https://github.com/amanosatosi/taiyou/actions/runs/40324826063),
job121578730470, artifact12650838111. This is NOT a kept final checkpoint.
All2604 primary samples complete; zero process failures, candidate/parent RGBA or
diagnostic changes, original Taiyou benchmark digest changes, or cache OFF/ON
normal-resource result changes. Native ownership/cache/worker tests and exhaustive
publication RGB/alpha boundary checks pass. CI formatting patch is applied later.
Full gates remain deferred/failing for this explicitly marked draft experiment.

The experiment transferred the owned pixel allocation through Arc<Box<[u8]>> and
replaced the original per-pixel premultiplication test with a branchless full-buffer
reduction. Measurements do not justify retaining the combined version as-is:

| Workload/state | Retained parent1T | Experiment1T | libass RGBA |
|---|---:|---:|---:|
| Dialogue1080p warm |6.559|6.077|0.959|
| Borders1080p warm |9.411|10.988|2.947|
| Dialogue4K warm |20.769|18.902|4.524|
| Stress4K warm |80.012|73.220|93.409|
| Stress4K changing |409.775|401.223|101.281|
| Blur4K warm |26.036|22.624|10.236|
| Transforms4K changing |69.064|59.903|13.905|

The isolated publication probe also shows substantial pattern-dependent cost:
stress~13.88ms experiment vs~7.19ms parent; dialogue~13.90ms vs14.72ms. Do not
claim the branchless form was vectorized or publish one favorable median alone.
The next experiment removes the whole-buffer branchless reduction, tests cheap
all-zero blocks with the original validator on every nonzero block, and remeasures
allocation transfer against the same retained parent. Nothing is accepted merely
because all pixels match. Raw rejected experiment evidence stays intact here.

1t-table.md contains every workload/state ratio, painted-area/image-count evidence
and prior bottleneck attribution. JSON includes distributions, cold session/setup,
seeks and zero normal-resource differences. Selected post-sample text/CPU/RSS is
retained; all process reports remain in the remote artifact. Strict default 4K
stress span-work failures remain errors in both cache modes; production limits
were not raised. No local compilation, reference/tolerance or workload changes.
