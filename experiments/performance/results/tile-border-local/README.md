# Tile-local native unions: promising serial checkpoint

Candidate e405f8930c8a0e5058c12d672808a6e44804aa28; paired pre-tile reference
e1316cdd8f2eaea4d83b7a1ac3d08a4d0efd1762.
[Green run40365717076](https://github.com/amanosatosi/taiyou/actions/runs/40365717076),
job121722727445, artifact12670832905. Ubuntu24.04, EPYC7763,4 exposed CPUs,
Rust1.99.1. Pins/configuration/fonts/files/timestamps/counts and raw distributions,
painted areas/image counts are adjacent. Hosted-runner smoke is paired on one
runner, not controlled-hardware certification.

Adapted mechanism: libass ass_rasterizer.c:630..716 spatial subdivision and kernel
selection, with tile-local accumulation like c/rasterizer_template.h's generic
kernel. Taiyou keeps its own flattened geometry, discrete ellipse and native
eighth-grid arithmetic. No libass analytic coverage or stroke is copied. Compact
supports accumulate in small local row bitsets, with empty/solid shortcuts and
exact positive-union saturation. No expanded-span vector, global expanded-row
bucket/sort/union, or repeated global row search remains on admitted tile paths.

Paired medians in ms; external columns are direct API time:

|Workload/state|Pre-tile1T|Tiled1T|Stock direct|Mangetsu direct|Mangetsu RGBA|
|---|---:|---:|---:|---:|---:|
|Dialogue1080 cold|12.416|8.518|0.638|1.358|1.397|
|Dialogue1080 warm|3.882|3.831|0.062|0.196|0.263|
|Thick borders1080 cold|50.271|14.452|0.893|1.727|1.882|
|Dialogue4K cold|38.459|22.075|0.808|1.529|1.652|
|Dialogue4K warm|10.330|10.125|0.075|0.248|0.381|
|Stress4K changing|377.125|139.503|2.054|2.940|24.577|
|Stress4K warm|62.696|62.827|0.280|1.260|20.748|
|Blur4K cold|107.369|46.388|1.150|2.045|2.434|
|Blur4K warm|14.278|14.108|0.105|0.369|0.876|
|Transforms4K changing|51.731|32.690|0.828|1.140|1.722|
|Karaoke1080 cold|31.042|13.722|0.760|1.209|1.284|

Stress changing median falls63%; p95:397.198->143.426ms, p99:398.503->143.434ms.
The previous global-row-search tile candidate's ordinary regressions recover.
Warm cache-hit frames are essentially unchanged. Changing-sequence throughput
equivalent is2.65->7.17 FPS. Taiyou still takes5.68x the pinned Mangetsu RGBA core
time here; serial work is not complete. Host composition is recorded separately.

All4340 primary frames complete;868 candidate/reference pairs have equal RGBA and
diagnostic SHA-256 and painted counts (sample-audit.json). Further changing profiles
also compare exact hashes. Native types/renderer/cache/seek/font/document/retained-
frame/resource/worker tests and Clippy pass. Normal production cache OFF/ON outcomes
agree, including the4K stress LimitExceeded/Other rejection. Original benchmark
Taiyou pixels stay unchanged. No reference/tolerance/worker/dependency/cap changes.

One further stress profile avoids4,238,383 expanded intervals, generates0, and uses
1,805,856B optional tile scratch. The original quota witness remains248,414,936B /
3,443,207 work units, including failure ordering. All new storage must fit existing
available transient memory or fall back. The witness is not an RSS claim. Raster
capacity growth is~2.07MB; those counters cover the retained producer, with tile
storage reported separately. Post-sample costs: borders18.479ms, effects54.666ms,
composition45.724ms, base outlines12.798ms. These are further profile timestamps,
not the primary sample population. Filter/composition now cost more than borders.

Decision: retain for full validation because heavy and ordinary cold/changing
workloads improve materially without a significant warm regression. Historical101-
frame and Windows/Linux/macOS validation are pending on the unmarked checkpoint.
This probe remains non-mergeable; independent external oracle differences remain
failures. CI formatting is recovered/applied; no local compilation occurred.

## Full unmarked checkpoint

dc14d7a6ac65ee5268e58e3eb0b3ff2766869950 passes the complete
[platform run40367862277](https://github.com/amanosatosi/taiyou/actions/runs/40367862277),
including Windows job121730473246, macOS121730473176, Linux121730473343 and
Platform gate121740767833. The Linux recipe includes native MSRV and sanitizer
validation. [Fast40367862285](https://github.com/amanosatosi/taiyou/actions/runs/40367862285)
and documentation/T017 checks pass.

The [full comparison40367862385](https://github.com/amanosatosi/taiyou/actions/runs/40367862385),
job121730452012, artifact12670764104 preserves all101 recorded Taiyou raw frame
digests, with0 changed/missing/added (frame-audit.json). The independent oracle
remains red:1 exact frame,100 existing pixel mismatches,0 unsupported visual
frames/crashes/infrastructure failures. Two parser safety probes and one
upstream-disabled source are separate. T016's same five mismatches remain red.
Corpus policy still requires the human owner's exact-head/base/change-digest
acknowledgment; no agent acceptance or mergeability is claimed. This freezes the
validated raster checkpoint while further serial performance work continues.
