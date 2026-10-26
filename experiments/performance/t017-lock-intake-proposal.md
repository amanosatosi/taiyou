# Narrow historical-performance dependency intake proposal

The multithreaded checkpoint uses Rayon1.11.0 and its Actions-resolved packages,
plus the already resolved SHA-256 package directly in the benchmark. The existing
T017 history guard permits only the older DBCS codec addition, so the full Linux
workspace lane stops before its historical/current performance experiment:

`dependency change exceeds pinned optional DBCS codec intake`

Evidence: [Linux job](https://github.com/amanosatosi/taiyou/actions/runs/40303684848/job/121507937721).
The actual new native cache/worker tests and focused measurements run separately.

The maintainer explicitly authorized this exact patch in the PR36 performance
conversation on2026-10-26. It is now applied to `experiments/t017/historical.py`;
Actions verification is pending. Authorization is limited to this hash pair and
the associated negative probes, with no additional harness refactoring.
It adds exactly one normalized old/new Cargo.lock SHA-256 pair. All existing
dependency checks remain the fallback; changed checksums, versions, unrelated
packages and even unreviewed lock-byte edits still fail. Three additional
negative probes cover altered bytes, pool version and codec version.

The proposed intake is the reviewed historical lock plus encoding_rs0.8.35,
rayon1.11.0, rayon-core1.13.0, crossbeam-deque0.8.8, crossbeam-epoch0.9.21,
crossbeam-utils0.8.23, either1.19.0, and the explicit render/CLI edges in the root
Cargo.lock. No prior package versions change. Exact registry checksums are in
that lock; licenses/MSRV and purpose are in docs/DEPENDENCIES.md and the saved
Actions dependency metadata.

No historical number, manifest, script/font/source hash, pixel assertion,
reference image, comparison tolerance, corpus coverage or failure classification
changes. Historical environments remain labeled historical; candidate benchmarks
must still execute. This is a separate maintainer authorization request because
the existing validation guard is deliberately narrower than the current phase.
