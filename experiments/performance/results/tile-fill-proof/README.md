# First recursive tile fill: native coverage proof, not production selection

Commit ee5cb53d89bbb1f690dcb31553d674399d02d6f6;
[green focused proof40351735523](https://github.com/amanosatosi/taiyou/actions/runs/40351735523),
artifact12664521090. Two supplemental tests pass (60 closed polygon/phase/offset
cases plus decline probes),2.54s. CI formatting patch is retained and applied.
The first1e45aea dispatch40351143456 had a YAML scalar error before any test;
corrected provisioning syntax, no test or expected outcome changed.

The test-only producer retains integer line segments, partitions segment references
recursively into16x16 pixel tiles, visits left/top first and carries exact native-row
winding. Solid/empty regions bypass generic fill, single-segment leaves avoid sort,
and complex leaves sort only local crossings. Integer16.16 crossings and native
0..64 accumulation preserve Taiyou's existing discrete coverage, not libass's
analytic A8 approximation. Scratch is request-local, explicitly bounded and
fallible; results own their bytes. No native handle, global state or worker change.

Coverage bytes and original source/merged interval counts match the unchanged
producer across fractional phases, negative/integer placements, holes, overlapping
contours and self-intersections. All four kernel categories are exercised. Open
contours, unproved extents and insufficient optional scratch decline. Production
rendering is unchanged: module is compiled only in native unit tests. No renderer
speed claim is made, and no full historical audit or platform checkpoint is claimed.

Next: prove the old deterministic resource witness/rejection order, share the
existing curve flattening, then integrate the narrow closed-fill producer and
measure it in the same-runner Taiyou/stock/Mangetsu harness. Borders remain on the
old path until that proof, followed by removal of expanded-span work. This is an
intermediate architecture experiment, not completion of the performance task.
