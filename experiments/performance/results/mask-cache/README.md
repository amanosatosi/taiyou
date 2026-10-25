# First cache layer: paired measurements

Candidate `3faa1f6cff45b7ffcbcee87af1d193c7d77d4785`;
[run](https://github.com/amanosatosi/taiyou/actions/runs/40240312875),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40240312875/job/121310499805),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40240312875/artifacts/12616938403).
Same binary/runner, sequential OFF then ON processes, EPYC 7763 with four logical
CPUs. This is hosted smoke. Setup/parse/font admission are excluded from renders.
Native stages, three cache unit tests and all three supplemental cache integration
tests passed. Integration fixtures require exact successful RGBA/diagnostic equality
at 1080p/4K, not tolerances or accepting paired failures as visual matches.

Repeated timestamp medians (ms):

| Workload | Cache OFF | Mask cache ON | Hit rate | Retained KiB |
|---|---:|---:|---:|---:|
| Dialogue 1080p | 42.77 | 6.69 | 100% | 91.06 |
| Dialogue 4K | 164.26 | 23.03 | 100% | 206.04 |
| Thick borders 1080p | 716.25 | 11.43 | 100% | 105.29 |
| Large blur 1080p | 229.96 | 30.72 | 100% | 95.68 |
| Large blur 4K | 956.10 | 103.37 | 100% | 224.45 |
| Combined 1080p | 1863.48 | 263.31 | 100% | 320.28 |
| Combined 4K | 7838.54 | 967.63 | 100% | 1114.61 |

Cold, sequence and seek samples, counts, p95/p99, misses and evictions are retained
in the CSV/JSON, including lower hit rates on changing geometry. The renderer's
output is never a cache entry. Integer shadow relocation and duplicate geometry
also reduce cold-frame work; cold is still appreciably slower than a warm frame.
The raw time-capped sequence groups can cover different numbers of timestamps;
their distributions are exploratory rather than an equal-length sequence speedup
claim. A fixed-sequence comparison is still required for the final table.

Cache ON mask work falls to ~0.08 ms for dialogue 1080p. Large-blur 4K retains
~72.31 ms filtering, and combined 4K retains ~764.22 ms filtering out of 967.63 ms
total. Composition in the combined 4K case stays ~183.7 ms (versus ~183.1 ms off),
so no evidence of an unexpected composition cost increase. A separate complete-
input blur-result cache is justified next; a whole shaped-run cache is not.

Process peak RSS: OFF 216472 KiB, ON 218008 KiB across their respective full
selected runs; this includes setup and cannot isolate allocator/native memory.
Index/key/mask retained bytes are separately counted. No hard timing gate exists.

The measurement job remains **red**, because thick borders at 4K exhaust the
benchmark-only 512 MiB allowance in both modes. Both failure files are preserved.
Production-default span/work rejections are also unchanged and separately recorded.
No comparison reference, tolerance, inventory or failure interpretation changed.
