# Completed optimized T017 experiment — 77afbd2

Measured candidate `77afbd20be79b5a5c4863fe55eff79d3fefa9e30`; production sources equal `ae467c9`.
Earlier=`9eee96fe16989089d8fd3f65b1bb136455ffca7e`; reviewed=`027e58a52efdd870b99f49ab5f3bae6e952f8f59`.
[Source Linux native/sanitizer job](https://github.com/amanosatosi/taiyou/actions/runs/39288634496/job/118183558546).
144 recorded attempts:126 completed frames,18 genuine default-limit failures. Three samples per configuration. Original JSONL, scripts, environment, build/instrumentation identities and logs are adjacent.

Release opt-level3, sequential cold CLI processes; wall/CPU include font intake and output encoding. Timers overlap. Process RSS is distinct from conservative logical reservation bytes. Hosted-runner observations do not establish guaranteed playback FPS or universal speedups.

| Workload | Earlier ms | Reviewed ms | Current ms | Current RSS KiB | Current transient B | Current local B | Current CPU s | Eval/shape reviewed → current |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| normal-one | 10.899 | 10.706 | 6.782 | 5944 | 14989167 | 16503 | <0.01 | 2/1 → 1/1 |
| normal-eight | 46.080 | 48.569 | 14.832 | 5984 | 15107736 | 132024 | 0.01 | 24/16 → 8/8 |
| dense-100 | 61.139 | 88.322 | 16.428 | 6064 | 15349567 | 426700 | 0.01 | 300/200 → 100/100 |
| dense-500 | 482.746 | 419.228 | 60.098 | 6896 | 17230352 | 2133500 | 0.05 | 1500/1000 → 500/500 |
| dense-1000 | 2134.886 | 833.061 | 117.349 | 8052 | 19581352 | 4267000 | 0.11 | 3000/2000 → 1000/1000 |
| mixed-2000 | 1088.598 | 1123.990 | 247.753 | 7060 | 15291516 | 23415 | 0.24 | 4006/2004 → 2002/2002 |
| positioned-5000 | 2725.269 | 2809.949 | 609.132 | 9188 | 15744327 | 5007 | 0.60 | 10000/5000 → 5000/5000 |
| adaptive-256-multiglyph | 2349.547 | 2402.964 | 512.749 | 7952 | 23235683 | 8096768 | 0.50 | 768/512 → 256/256 |
| typesetting-100-border-0.5 | 558.717 | 562.699 | 159.449 | 6100 | 15207440 | 39422 | 0.15 | 202/102 → 100/100 |
| typesetting-100-border-3 | 621.344 | 627.721 | 225.318 | 6308 | 15460322 | 39422 | 0.21 | 202/102 → 100/100 |
| typesetting-500-border-0.5 (4M caller segments) | 2790.392 | 2808.607 | 780.277 | 6760 | 15973040 | 39422 | 0.77 | 1002/502 → 500/500 |
| typesetting-500-border-3 (4M caller segments) | 3120.886 | 3141.708 | 1113.481 | 7376 | 17255522 | 39422 | 1.10 | 1002/502 → 500/500 |
| typesetting-hd-100 | 797.060 | 804.995 | 398.178 | 24788 | 30900616 | 54020 | 0.38 | 202/102 → 100/100 |

Small scenes: PlayRes/output160×100, WrapStyle2, unchanged pinned Source Sans3 Regular3.052. The HD scene uses1920×1080,98 explicit fs32/bord2/visible-shadow2 multiglyph events plus two automatic dialogue events. Original scripts, hashes, exact font pin, compiler/runner/CPU/native-prefix identities are recorded in environment.json and sample records.

## Retention strategies, same approved F1 policy

| Strategy (256×32 glyphs) | Shapes (all3 samples) | Median ms | RSS KiB | Transient B | Aggregate local B |
|---|---:|---:|---:|---:|---:|
| candidate | 256 | 512.749 | 7952 | 23235683 | 8096768 |
| candidate-adaptive-32MiB | 439 | 534.949 | 6888 | 17447759 | 2340472 |
| candidate-minimal-32MiB | 512 | 543.542 | 6880 | 16417346 | 31628 |
| candidate-minimal-128MiB | 512 | 542.965 | 6860 | 16417346 | 31628 |

No successful partial-retention sample replayed the complete frame. Full auto-only retention spends more memory than minimal staging; position-dominated scenes preserve the large original memory reduction. The earlier640-call quarter-admission result remains in2574e17-partial.

## Genuine default-limit failures

The default1M segment budget fails in all three snapshots for both500-event multiglyph scenes. Thin: event441,1964 requested output points,834 remaining segments. Thick: event235,1975 output points,714 remaining. Every failure has exit2 and no raw frame/hash. The candidate's bounded fallback also retries these non-memory limit failures once (888 thin /476 thick preparation calls); these totals are failed attempts, not completed-frame performance. No limit or production default changed. With explicit4M caller segments, all18 attempts completed and each workload's exact RGBA matches across all three generations.

## Independent equivalence

All successful reviewed/current/staging-variant hashes agree. All generations agree for unchanged Core behavior. The sole pre-F1 difference is adaptive-256-multiglyph: earlier `266aef8ede868b0b3d5dfbbe52dd62ed0e890b242ec2372afdd5bd1de4a79ad8`, F1/current `8d72e9f87c466e695feca177435455a667a8a499965186e17bb4ad8c176b6878`. The new independent fixed-font F1 test passes on Linux/macOS/Windows: union analytic per-glyph radius0.5/shadow1.25 hulls, derive touching anchors, compare real output against those explicit anchors and require inequality with the old local-box anchors. No expected pixels were changed.

## Inclusive stage profile (median ms)

| Case | Evaluation | Shaping | Layout | F1 footprint | Collision | Outline | Geometry | Raster | Stroke | Shadow | Composition | Font resolution | Native shaping |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| normal-eight | 0.007 | 0.744 | 0.029 | 0.028 | 0.002 | 6.200 | 6.465 | 0.888 | 0.000 | 0.000 | 0.155 | 0.009 | 0.670 |
| positioned-5000 | 5.616 | 289.751 | 5.696 | 0.000 | 0.000 | 249.562 | 257.432 | 4.939 | 0.000 | 0.000 | 0.098 | 5.683 | 265.633 |
| adaptive-256-multiglyph | 0.251 | 21.059 | 1.446 | 2.548 | 0.410 | 343.036 | 353.799 | 12.760 | 102.019 | 53.227 | 0.371 | 0.274 | 18.481 |
| typesetting-hd-100 | 0.207 | 11.513 | 0.457 | 0.008 | 0.001 | 84.238 | 87.900 | 23.688 | 152.607 | 75.821 | 42.986 | 0.184 | 10.274 |

Repeated full font-byte checks through authenticated catalog operations:0; initial catalog authentication is still mandatory. Geometry includes outlines; shaping includes native work; shadow includes a possible stroke. These columns cannot be added as disjoint stage times. Positioned5000 has zero footprint and collision calls; remaining cost is actual native shaping/outline extraction, not collision. No native-context/global cache was added.

The complete48 debug samples fromae467c9 were hash-verified and reused, explicitly labeled with their original SHA/environment/job. No historical180-second timeout or unchanged expensive debug baseline was rerun. All ordinary gates, sanitizer and full comparator still ran on77afbd2.

