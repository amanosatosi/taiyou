# Second measured cache layer

The first cache removes most repeated span work. Its paired run leaves 764 ms of
filtering out of 968 ms for repeated combined 4K output, so filter-result retention
is a demonstrated next target rather than speculative parity with libass.

`CacheConfig::Bounded` now also retains filtered masks in the same bounded budget
and deterministic LRU. `MasksOnly` independently bypasses this layer for paired
measurement and debugging. Disabled/zero-budget operation keeps the ordinary
producer. Inactive and pointer-identical no-op filter results are not retained.

The complete key includes source mask bytes, stride, dimensions, coverage units,
resolved Gaussian sigma/radius, edge passes/fractional coefficients/padding,
producer/profile revision and fixed resource policy. Input key bytes are owned
and charged separately; no dependency on mutable native state or document IDs.
Integer origin is factored out only because filtering samples local array
coordinates; checked relocation preserves output bounds and bytes. Source data
and all filter parameters must compare equal; hash equality is never sufficient.

The existing combined extent/scratch preflight runs on hits too. Resource errors
therefore remain errors; cache state cannot bypass allocation/extent limits.
Tests vary raw input bytes, coverage units, offsets and parameters, compare full
mask bytes/bounds/stride with fresh filtering, and exercise a low-memory hit.
Full-frame exact OFF/ON fixtures from the first layer continue unchanged.

The focused Actions run compares mask-only and all-cache modes sequentially on one
runner. The committed original baseline is not rerun for this layer. Production-
default failures stay separately visible; the previously failing 4K thick-border
benchmark remains preserved in earlier complete reports and final coverage.
No existing correctness/comparison suite is selected down or modified.
