# Expanded uncached baseline

Candidate: `0006774a549d362aea3e26912cc8ce511fc108b2`.
[Run](https://github.com/amanosatosi/taiyou/actions/runs/40237780495),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40237780495/job/121303002900),
[original artifact](https://github.com/amanosatosi/taiyou/actions/runs/40237780495/artifacts/12614879871).

105 of 110 workload/resolution/playback combinations produced samples. The five
4K thick-border scenarios fail with transient memory pressure even at the explicit
512 MiB/32-million-work-unit experiment settings. The job correctly remains red.
Production-default preflight rejections are separately listed in `failures.tsv`;
no limits, pixels or regression classifications were changed.

This run used an EPYC 9V45 VM, unlike the initial EPYC 7763 partial run. Do not
interpret their timing differences as an optimization. This is hosted smoke,
not controlled hardware. Same-time median observations (ms):

| Workload | 1080p | 4K |
|---|---:|---:|
| Dialogue | 28.44 | 105.00 |
| 12 simultaneous events | 298.01 | 1159.97 |
| Vectors | 37.41 | 151.80 |
| Thick borders | 465.32 | resource failure |
| Large blur | 148.22 | 607.62 |
| Animated transforms | 52.85 | 206.60 |
| Combined abusive case | 1206.80 | 5080.83 |

Masks dominate. For the combined 4K case, mask preparation is 4972.40 ms, of which
467.45 ms is blur/effects; composition is 98.27 ms. A further scan/stroke/union/
coverage timing split precedes selecting a significant cache. Native stage tests
and profiling-on/off pixel/diagnostic equality checks passed for rendered warm-up
cases. Raw samples and actual counts are preserved; short heavy-case distributions
are not robust tail estimates. Process peak RSS is 217740 KiB for the entire run.
