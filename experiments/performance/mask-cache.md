# First measured cache layer

The pre-cache split identifies span union as the dominant cost. The renderer now
has opt-in `CacheConfig::Bounded` retention, using the caller's `Limits::cache_bytes`.
The existing default remains disabled for this first implementation checkpoint.
The new store retains fill, ordinary border and translated shadow coverage masks,
not RGBA frames, paint, clips, diagnostic reports or native objects. Blur continues
to run independently so its next measurement is not hidden by this cache layer.

Keys compare the complete ordered Q6 path after factoring out whole-pixel origin,
body/border producer identity, exact border axes and the fixed resource policy.
Subpixel phase remains in every coordinate. Current CoreV1/nonzero/native-64-unit
producer semantics are fixed by this typed store; no persisted cache exists.
Upstream font/realization/transform differences reach this boundary as actual path
differences. A same-name font or coincident catalog revision cannot alias geometry.
No pointer, timestamp, color or hash-only identity is used. Bounded linear lookup
checks full values; there is no hash-collision correctness dependency.

Whole-pixel relocation is permitted only when the entire normalized path matches.
The existing scan converter itself factors out the same integer hull origin before
curve evaluation. Fractional paths retain exact Q6 differences. Checked relocation
of a hit copies only the coverage plane; unchanged placement shares the immutable
mask. Failure to relocate/admit falls back to ordinary production. Native projected
border stroking remains an explicit bypass, with all existing rendering available.

The producer records deterministic peak byte/work checks from its existing
allocation/segment checks. A hit still validates/builds the input path and must fit
the same required transient/work reservation. These resource witnesses are not
wall-clock observations. Low remaining work or memory therefore cannot turn a
previous error into a cached success. Fixed caps stay in the key; no production
limit is increased. Resource-failure equivalence has focused tests.

Retention is bounded by both bytes and 128 entries. Accounted bytes include fixed
index capacity, owned canonical path capacity and mask allocations. Key storage
does not retain font/catalog/track owners. Deterministic request-order LRU evicts
only unpinned masks; pinned masks remain indexed and charged. Oversized entries
bypass admission without flushing the cache. Zero/tiny budgets are valid, admission
allocation failures bypass, and producer failures are never retained. Completed
frames independently retain only their existing RGBA/output-budget ownership.

Focused counters report hits, misses, evictions, bypasses, entries and retained/
peak bytes. Exact RGBA plus diagnostics are checked against uncached fresh renders
over static/animated text, vectors, borders, blur, karaoke, clips, seeks, fonts,
documents, 1080p/4K, tiny budgets and retained-frame lifetime. The paired benchmark
uses sequential off/on processes on one runner with the same binary/inputs; setup
and parsing remain outside render timings. Existing frozen comparisons are intact.

Review surfaces: additive cache configuration/statistics API, canonical path
dependency proof and deterministic resource-witness checks. This is a proposal
for engineering review, not a claim of maintainer acceptance or full T019 completion.
