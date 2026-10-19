# T017 P0 and prerequisite measurements

Measured production source: `dc83cab0be5bbb86d1a994956f77a97a08e9716b`. [Actions job](https://github.com/amanosatosi/taiyou/actions/runs/39300717499/job/118225559837), artifact12211923565, ZIP SHA-256 `900ca05f2fab1886b59074227bb29b3e538875358cb82b6362b245706d0842d7`.

All benchmark assertions completed. The overall job then failed because the new synthetic P0 unit fixture had overlapping operation provenance. That fixture is corrected separately. This job is **not** platform/sanitizer acceptance. The existing source was not changed to excuse that failure. Original debug48 and release144 records,18 default-limit failures and the old180-second timeout remain intact. Historical rows are explicitly reused, current candidate rows freshly executed.

Release opt-level3, three cold CLI processes per case/configuration, pinned Source Sans3.052, identical native dependency pins. Historical runner/environment data is retained in historical-reuse.json; these are not simultaneous same-run baseline pairs. CPU/RSS/stage counters and script/frame hashes are in samples.jsonl. The earlier optimized77afbd2 has the same production sources as reviewed f41261d (only an additive search test differs).

| Workload | Original9eee ms | F1 reviewed027 ms | Optimized77 ms | Currentdc ms | Current RSS KiB | Transient B | Eval / shape |
|---|---:|---:|---:|---:|---:|---:|---|
| adaptive-256-multiglyph | 2349.547 | 2402.964 | 512.749 | 514.216 | 7984 | 23235683 | 256 / 256 |
| dense-100 | 61.139 | 88.322 | 16.428 | 16.162 | 6116 | 15349567 | 100 / 100 |
| dense-1000 | 2134.886 | 833.061 | 117.349 | 118.313 | 7976 | 19581352 | 1000 / 1000 |
| dense-500 | 482.746 | 419.228 | 60.098 | 60.867 | 6924 | 17230352 | 500 / 500 |
| mixed-2000 | 1088.598 | 1123.99 | 247.753 | 253.032 | 7032 | 15291516 | 2002 / 2002 |
| normal-eight | 46.08 | 48.569 | 14.832 | 14.956 | 5864 | 15107736 | 8 / 8 |
| normal-one | 10.899 | 10.706 | 6.782 | 6.905 | 5704 | 14989167 | 1 / 1 |
| positioned-5000 | 2725.269 | 2809.949 | 609.132 | 615.681 | 9140 | 15744327 | 5000 / 5000 |
| typesetting-100-border-0.5 | 558.717 | 562.699 | 159.449 | 479.386 | 6088 | 15207440 | 100 / 100 |
| typesetting-100-border-3 | 621.344 | 627.721 | 225.318 | 229.53 | 6216 | 15460322 | 100 / 100 |
| typesetting-500-border-0.5 | 2483.614 | 2484.735 | 1369.168 | 698.645 | 6684 | 15865436 | 444 / 444 |
| typesetting-500-border-0.5-4Msegments | 2790.392 | 2808.607 | 780.277 | 787.31 | 6672 | 15973040 | 500 / 500 |
| typesetting-500-border-3 | 1549.341 | 1484.232 | 1041.504 | 530.224 | 6632 | 16081490 | 238 / 238 |
| typesetting-500-border-3-4Msegments | 3120.886 | 3141.708 | 1113.481 | 1113.613 | 7332 | 17255522 | 500 / 500 |
| typesetting-hd-100 | 797.06 | 804.995 | 398.178 | 395.314 | 24660 | 30900616 | 100 / 100 |

Default500 thin/thick rows are failed attempts, not completed frames: exit2/LimitExceeded, null RGBA hash. Thin: event441,1964 output points with834 segments left; thick: event235,1975 points with714 left. Current444/238 preparations replace previous888/476. Explicit4M caller-budget rows complete500 events with exact cross-generation/staging RGBA agreement. Production defaults remain1M.

The thin100 cold-process samples were159.201,479.386 and595.256ms, despite identical100 shapes/98 strokes and0.15CPU seconds each. Renderer-stage times stayed near their earlier values. The extra wall delay is not localized to renderer work; scheduling/process/output-I/O causes are possible inferences, not proven causes. Keep this observed deterioration visible. A separate bounded warm-render/seek experiment measures only Renderer::render and checks every frame against a fresh renderer; no speedup is inferred from these cold samples.

| Staging256 | Shape calls | RSS KiB | Transient B | Local B |
|---|---:|---:|---:|---:|
| candidate | 256 | 7984 | 23235683 | 8096768 |
| candidate-adaptive-32MiB | 439 | 6780 | 17447759 | 2340472 |
| candidate-minimal-32MiB | 512 | 6828 | 16417346 | 31628 |
| candidate-minimal-128MiB | 512 | 6764 | 16417346 | 31628 |

All successful current/reviewed frame hashes match; full/partial/minimal retention is exact. The pre-F1 multiglyph hash difference remains the independently tested approved F1 footprint change, not a new P0 difference. RSS is process high-water memory, not logical reservation accounting or a per-stage allocation measurement.

## Raw native-work probe

Debug, pre-registered four-font catalog, one complete warm test-process measurement per case. Exact UTF-8 input files are adjacent and their hashes are in native-work-completed.json. The raw-work probe and earlier width probe are different workloads/configurations: notably this spaced-CJK case requests Source Sans3 as primary, forcing CJK/space fallback groups. The earlier unspaced long-CJK test requests Noto Sans CJK JP and completes in7.408s. Do not compare those as a same-input regression.

| Workload | Candidates | Native attempts | Context bytes | Raw glyphs | Total s | Width-table B | Reusable search ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| latin-long | 20910 | 20910 | 10757860 | 10757860 | 29.262 | 669120 | 0.485 |
| cjk-long | 25425 | 3923700 | 1758299100 | 19110900 | 188.735 | 813600 | 0.574 |
| mixed-bidi | 1176 | 71472 | 16124360 | 263840 | 10.53 | 37632 | 0.061 |
| pathological-combining | 1176 | 1176 | 136024 | 57624 | 0.299 | 37632 | 0.04 |

The shaping column in raw JSON covers the whole font shaping API, including resolution/fallback/cluster assembly, not exclusively hb_shape. Each candidate is evaluated once; DP reuse does no native shaping and assumes no monotonic widths. No ASS parse/evaluation is repeated per candidate. The retained cost table is dense Option<f64> storage in this experiment, not an admitted production allocation policy.

## Limit recommendation for review, not defaults

Retain32000 candidates and4MiB candidate-width storage as bounded experiment proposals; add131072 native attempts,64MiB cumulative context bytes,64MiB range bytes and16M raw native glyphs per event as values for the next caller-budget trial. These cover measured long-Latin, mixed-bidi and combining totals and intentionally reject the3.9M-attempt fallback workload. They are not approved or validated production defaults, and no runtime guarantee is extrapolated from counts. Long Japanese/CJK with selected primary/fallback families, Arabic/Indic-only raw work, full boundary costs, storage preflight and font-driven output expansion still need bounded-budget trials and C7 review. Never replace exhaustion with greedy fallback or a partial frame.

Raw glyphs can be charged only after HarfBuzz returns; calls/context/range bytes are preflighted. Existing per-call scratch/glyph checks remain. A complete production guarantee for excessive substitution expansion has not been established by this prototype. The current public renderer never enables these experimental budgets or automatic WrapStyle0/1/3. No new C7 default is activated.
