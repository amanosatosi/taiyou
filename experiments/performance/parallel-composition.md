# Measured parallel work: ordered row composition

Cached single-worker reference: `9cce24c9c07cfcc499b74af07339717c836d82da`.
The focused cache gate is green, all three platforms and existing Linux native
sanitizers pass, and the full comparison preserves all 101 observed Taiyou frame
digests. Existing libass mismatches remain red and are not new performance goals.

The combined 4K warm frame spends 163.12 ms of 186.97 ms in composition. This
justifies parallelizing that real renderer stage. A renderer owns one explicitly
sized Rayon pool. One worker uses the caller inline without a pool; default stays
one. Native font contexts, serial layout/collision decisions and cache mutation
stay on the coordinator. Workers only read immutable DrawOps/masks/clips and own
disjoint mutable RGBA row slices. Each pixel traverses the complete canonical
operation order with the existing integer blend function. Filters run on complete
masks before clipping and partitioning, so band edges cannot truncate halos.

Output extent, ordering, output-origin budget reservation and transient preflight
all run before dispatch in the same order as scalar composition. No per-band
operation lists, extra image planes or scratch buffers are allocated. Shared
pool/runtime memory is not claimed to be a hard process-RSS cap. The existing
native sanitizer lane is not a Rust race detector; safe slice ownership prevents
overlapping pixel writes, and no native handles enter jobs.

`set_composition_band_rows` is an explicit batching control (default provisional
64 rows); benchmarks can examine 16/32/64/128 without environment reads in the
renderer or schedule-dependent choices. Configured workers are verified against
the created pool; no global pool or automatic worker policy. Tests compare all
batch sizes and 1/2/4/8 workers, seek/resource/font changes, clips and coverage
units. No automatic serial threshold is selected before measurements.

The first implementation intentionally leaves raster misses and blur producers
serial: duplicating native state or adding a dependency scheduler without evidence
would expand the safety boundary. Further parallel work must follow the measured
remaining bottleneck. This is the T025 composition part of the requested phase,
not a claim that every planned T024 preparation task has been implemented.
