# Mask-stage split before caching

Candidate `4217a1eee8dfc45cbc877b5ea1583e42e51b9777`;
[run](https://github.com/amanosatosi/taiyou/actions/runs/40238945073),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40238945073/job/121306399732),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40238945073/artifacts/12616365847).
Hosted smoke with the explicitly recorded benchmark allowances; production caps
unchanged. Selection is recorded. Native render stage tests passed. The job is
red because thick borders at 4K still exhaust the experiment memory allowance.

Same-time median milliseconds, with short heavy-case sample counts in the JSON:

| Workload | Total | Base outline extraction | Scan conversion | Border expansion | Span union | Mask accumulation | Blur |
|---|---:|---:|---:|---:|---:|---:|---:|
| Dialogue 1080p | 27.11 | 0.92 | 2.19 | 3.17 | 17.61 | 0.34 | ~0 |
| Dialogue 4K | 103.14 | 0.94 | 4.45 | 12.67 | 74.94 | 1.04 | ~0 |
| Thick borders 1080p | 454.57 | 2.82 | 6.16 | 46.83 | 393.27 | 1.66 | ~0 |
| Large blur 4K | 604.91 | 1.90 | 8.45 | 74.61 | 461.09 | 2.82 | 43.58 |
| Combined 4K | 4929.41 | 7.63 | 36.57 | 549.95 | 3740.08 | 33.48 | 450.09 |

Mask subtraction/combination is insignificant in these opaque fixtures. Base
outline time is a subset of geometry. Scan/expansion/union/accumulation and blur
are subsets of the mask aggregate; do not sum these again with their parent.
Projected native border fallbacks remain inside the aggregate rather than gaining
speculative instrumentation inside the native adapter.

**First cache choice:** immutable paint-independent fill/border masks after span
union. Union is the dominant reusable computation. Retaining the compact final
mask also avoids accumulation and is smaller than retaining the expansion's
large temporary span capacity. Keys must preserve full path/phase/border inputs;
integer relocation requires exact translation invariance and checked bounds.
Blur results remain a separate subsequent measured layer. No whole-frame cache,
shaping-first optimization or rendering-semantic change is proposed.
