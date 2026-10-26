# Legacy resource witness proof before tiled production selection

Commitef7f255ecb130724138275b355d646e5692d7a76;
[green focused run40353353294](https://github.com/amanosatosi/taiyou/actions/runs/40353353294),
artifact12665396233. The two tiled geometry tests pass with added resource checks
(4.34s), plus the exact runtime vector-capacity history test (one pass). Native
build, existing renderer slice and Clippy pass. CI formatting is retained/applied.
No production raster selection changed in this commit.

Across60 phase/polygon/offset witnesses, computed peak bytes and required work
match the legacy producer. Eight additional memory/work policies per witness
compare success/witness or exact rejection code, detail and resource category.
The capacity model matches actual reserve-one growth through8192 elements for
Edge, Span and ellipse i64 stores on this runner. Keep this proof in platform/MSRV
validation: a standard-library growth change must not silently alter old policy.

The model preserves the first failing push (arithmetic, transient, work checks),
old source/expanded capacities and final mask charge; it does not charge future
tiles as if they still materialized expanded spans. The old conservative quota
witness remains the cache-hit preflight even when physical raster scratch shrinks.
Optional scratch inability declines to the retained producer, not a new error.
Full101-frame and cross-platform gates remain required at a promising checkpoint.
