# R002 independent coverage experiments

Research only; no production backend or accepted reference is installed.
All commands below run **only in GitHub Actions**, never on the user's PC.

The additive R002 workflow uses the existing verified native acquisition/build
recipe, then `python experiments/coverage/run.py`. Existing tests and full T015
comparison run independently. A standalone temporary Cargo driver depends on the
unchanged local production crates. Its dependency entries must match the existing
Cargo.lock exactly; it introduces no package dependency or root lock change.

baseline.rs calls the actual T012 geometry/group/raster APIs. Raw .a8 bytes,
Q6 geometry Debug dumps, bounds/origins/stride, backend settings, initialized PGM
views and 30 raw warmed timing samples per case are retained in R002-Core-A8
artifacts. Each sample averages 20 calls; context construction and file writing
are excluded. Two fresh driver processes must yield identical bounds/stride/bytes.
The reported allocation is retained A8 bytes, not native scratch, peak heap or RSS.

stroke.c uses public FreeType 2.14.3 APIs, closed native y-up rectangle/acute
contours and round joins/caps. Radii are Q6 0,16,64,1024. This is a native backend
probe, not Taiyou's nonexistent stroke implementation. Two processes repeat each
output. Raw CPU clock samples measure rasterization of the already widened
outline, not the widening operation or allocation. Do not compare their origins
to Core y-down coordinates without mapping. No xy numerical reference exists.

Grouping witnesses use actual T012 fill masks plus C6 integer alpha source-over.
Opaque and translucent grouped/separate results and absolute-difference PGM
are arithmetic comparisons under Core, not xy compatibility tests.
No alternate scan converter or widening implementation was justified/tested.

xy-probe.py executes only on Windows Actions. The official release archive is
discovered once per bounded session, inventoried with SHA-256/PE machine and kept
separate from any source commit claim. All DLLs are identified; only x64
VSFilter.dll is loaded. Original four-channel buffers and environment/hash JSON
are uploaded, without alpha extraction or golden acceptance. Failure/timeout is
not a visual pass. This experiment does not operationalize the PLANNED xy suite.

No binaries/fonts/corpus outputs are downloaded to the user's PC. Actions artifacts
retain raw outputs for review (30 days); small measured summaries/provenance may
be checked in after inspection, never fabricated. Candidate SHA/run/job/artifact
identity belongs in the research execution record. Promotion requires separate
review, authenticated capture provenance and ADR-011 acknowledgment.
