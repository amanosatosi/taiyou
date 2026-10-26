# Expanded-span removal: material stress gain, ordinary regressions remain

Candidate b9b3c150345ae23f3cdbb897d0ef195fa598cb21; same-runner pre-tile reference
e1316cdd8f2eaea4d83b7a1ac3d08a4d0efd1762.
[Run40363446330](https://github.com/amanosatosi/taiyou/actions/runs/40363446330),
job121714689731, artifact12668943729, green. Ubuntu24.04, EPYC7763,4 exposed CPUs;
Rust1.99.1. Full environment/counts/distributions are adjacent. Original stock and
Mangetsu pins and input files/fonts/timestamps/counts remain unchanged.

All4340 primary samples and the additional changing profiles preserve exact
candidate/reference Taiyou RGBA and diagnostic digests. Normal-limit cache OFF/ON
results agree; native tests, types, renderer slice, Clippy and cache/resource/
seek/font/document/retained-frame/worker tests pass. No reference/tolerance or
production limit changes. Formatting is recovered/applied from the runner.

Paired medians in ms, direct external API times (host composition is secondary):

|Workload/state|Pre-tile1T|Tiled1T|Stock direct|Mangetsu direct|Mangetsu RGBA|
|---|---:|---:|---:|---:|---:|
|Dialogue1080 cold|12.347|13.895|0.639|1.337|1.338|
|Thick borders1080 cold|49.937|22.525|0.875|1.769|1.841|
|Dialogue4K cold|38.417|47.440|0.818|1.518|1.630|
|Stress4K changing|376.834|214.673|2.118|2.931|24.012|
|Stress4K warm|63.345|66.107|0.282|1.203|20.720|
|Blur4K cold|107.997|66.478|1.158|1.955|2.071|
|Transforms4K changing|50.601|63.158|0.830|1.182|1.759|
|Karaoke1080 cold|30.860|24.483|0.764|1.237|1.324|

Stress changing p95:400.948->222.636ms; p99:437.523->238.103ms. All painted-area
ranges and external image counts remain in1t-table.md; the engines are not visual
oracles for each other. Primary timers exclude setup, hashing and stage profiling.

One post-sample stress frame avoids4,238,383 expanded intervals with0 physically
generated expanded spans. Its original resource witness remains248,414,936B and
3,443,207 work units; optional tile scratch is1,827,056B. Old expanded-row bucket/
sort/merge disappears, but the new border tile kernel costs89.978ms; effects56.956ms,
composition45.880ms, geometry13.756ms. Existing source sort remains0.689ms. These
are further profile timestamps, not the primary timing population.

Decision: provisional, NOT an accepted/full checkpoint. The43% changing-stress
reduction is material, but ordinary cold dialogue and changing transforms regress.
Inspection of the measured90ms tile kernel shows repeated global source-row
binary searches across ellipse distances inside already-partitioned complex tiles.
The next narrow candidate combines the tile-local compact support primitives
directly into native row bitsets. Keep/reject waits for exact paired measurements
and ordinary-workload recovery. Full101 historical and platform gates remain
required at a promising checkpoint; draft probe gates stay explicitly unsatisfied.
