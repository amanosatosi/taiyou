# Cache design

Normative with [C8](CONTRACTS.md#c8-concurrency-and-cache-boundary). Introduce caches only after uncached correctness. Every store supports bypass and statistics. No disk cache in v1, no pointer-address keys, and no global process cache.

## K1. Ownership and policy

Track parsing is explicit host-owned retention (`Arc<Track>`), not an unbounded hidden byte-to-track map. A catalog retains font bytes within its configured registration budget. A renderer owns typed stores for derived artifacts; shared values are immutable Arcs. Values may retain dependencies, but unique allocation accounting follows allocation owners, not every Arc edge. Each allocation has one charge released on final drop. Cache index removal does not imply memory was freed.

Initial 128 MiB retained-cache budget is shared, with soft quotas: shaping/layout 16 MiB, outlines 16 MiB, raster masks 48 MiB, strokes/effects 40 MiB, paint plans/frame plans 8 MiB. Font bytes, input IR, output frames and transient memory use separate documented limits. Quotas may borrow unused capacity, never exceed the total. Native face handles have per-worker count bounds (initial 16) and their measured memory is reported separately until a reliable allocator accounting path exists. Initial implementation uses no duplicate per-worker mask cache.

At barriers, evict least-recently-used entries by `(last_frame_epoch, last_request_ordinal, stable_entry_ordinal)`. Touches are merged in canonical request order, not worker completion order. Evict expired snapshots first as a preference, not a correctness rule. Pinned entries may be removed from the index but remain charged; prefer unpinned victims to reclaim memory. Oversized entries render transiently without admission if they fit the frame reservation; they do not flush every other cache. Zero budget is valid and disables retention.

Reserve all logical frame outputs and intermediate peaks before dispatch, in stable order. Release staging reservations as artifacts publish. Admission cannot make a previously successful uncached frame fail: evict/bypass retention first. A dependency DAG prevents deadlock when an output depends on another miss. Errors are never retained across frames except deterministic missing-font lookup within an immutable catalog revision; do not cache transient OOM. Cache loss changes cost, never pixels or diagnostic ordering.

## K2. Exact key dependencies

Every key includes a stage schema/policy revision and profile when semantics differ. Full equality is checked after hashing. Finite float bits are canonicalized; only the Q6 raster boundary and specified kernel coefficients are quantized. Keys store immutable descriptors or IDs whose registry owns the descriptor and never reuses IDs during a renderer lifetime.

| Store | Required key fields | Value / lifetime |
|---|---|---|
| Parsed track (host retention) | source bytes identity + encoding + parser revision + parsing options | Track; replaced by new snapshot on edit |
| Font resolution | catalog revision, family/weight/italic/vertical/encoding policy, cluster text/context, script/language | selected FontIds, fallback diagnostics |
| Shape | exact text and cluster/source map identity, resolved font sequence, feature settings, direction/script/language, variation coordinates, size/metric policy, spacing ligature policy, profile | glyph IDs, design-unit positions, cluster map and metrics |
| Local layout | shape keys, available width, wrap/alignment policy, line metric/spacing settings, margins where they constrain width, scale, break plan | line breaks, baselines, boxes; no absolute placement |
| Collision placement | complete ordered active eligible set, each ADR-012 footprint+anchor+layer+start, per-run geometry/effects/paint visibility, viewport/content transform, collision policy identity | placed boxes; never just one event's layout key; explicit positions excluded |
| Glyph geometry | font bytes digest+face index, glyph ID, variations, synthetic bold/italic/decorations, outline extraction flags/version | design-unit outline; no color or absolute position |
| Drawing geometry | normalized command sequence, p scale, baseline offset and fill rule | path; no paint |
| Transformed geometry | outline identity, full mapping including origin/3D parameters, size, viewport scaling, profile | Q6 contours with integer translation factored out only when equivalent |
| Fill raster | transformed geometry key, fill rule, coverage grouping identity, fractional x/y phase, raster backend/version/settings | A8 local mask |
| Border | base transformed geometry or base outline as selected by stroke policy, x/y widths in that space, joins/caps, stroke policy, phase, group union policy | expanded/outline mask; no paint color |
| Effects | source mask identities, group policy, exact real BE strength and resolved passes/fractional coefficients, Gaussian coefficients/support, target scale, pass selection/order, fractional shadow offset, effect policy revision | filtered A8 mask and support bounds |
| Clip | rectangle/path commands, inverse flag, clip mapping, viewport bounds for inverse, raster revision/phase | resolved clip mask/rect; not clipped source coverage |
| Paint plan | paint values/stops, alpha, paint space/map, karaoke boundary and cluster partition, color policy | solid parameters initially; no RGBA mask copy |
| Frame plan | ordered DrawOps, all placements/clips/paints/passes, viewport, profile | tile indices and structural changed-state; one previous plan retained within quota |

Shape outputs may use design units internally, but size is conservatively in the shape key until T009 proves a smaller dependency safe (e.g. optical sizing can be size-dependent). Initial geometry caches can store unhinted design-unit outlines across sizes; hinting remains off. Persistent hashes alone never identify font content: font registration verifies bytes and collection face index. Replacing a same-named font produces a new FontId/catalog revision.

## K3. Change matrix

“Reuse” assumes all other relevant inputs are equal. Recomposition still occurs whenever the final scene changes.

| Changed input | Earliest affected stage | Reusable artifacts / caveats |
|---|---|---|
| `\c`, per-channel alpha, fade | paint/grouping; collision placement if visibility changes an ADR-012 footprint | shape/layout/outlines; masks reusable if group partition unchanged; never reraster solely for RGB |
| `\pos`, `\move` | placement | integer translation reuses local masks when phase and mapping unchanged; fractional movement rerasterizes; explicit org, perspective, device paint or clip intersection can invalidate more |
| `\frz` | collision footprint and transformed geometry | shape/local layout and base outline; masks/stroke/effects rebuilt |
| `\fs` | shape/metrics | base design-unit outline may survive; layout/transform/masks rebuild |
| `\fscx`, `\fscy`, `\fsp` | shape policy/layout | spacing can disable ligatures; layout and geometry change; base outline reusable |
| `\bord`, x/y border | border and collision box | fill may survive; placement of other colliding events may change; all downstream affected placements re-evaluated |
| `\shad` | collision footprint and shadow placement/effect | fill/border unchanged; phase may require separate shadow raster; complete active-set placement recomputed |
| `\blur`, `\be` | effects | base fill/border masks survive; effect bounds and tile lists change |
| `\clip`, `\iclip` | clip | text geometry/effects survive; clipped final output changes |
| gradient colors/stops (future) | paint | geometry/masks survive; grouping may change; animation does not justify a geometry key |
| gradient coordinate space/map | paint mapping | masks survive unless geometry itself also changes |
| `\fn`, weight, fallback fonts | resolution/shape | unrelated fonts' geometry survives |
| Track revision | selection/evaluation | structural derived keys can reuse equal values; never reuse stale EventId across revisions |
| Output size/content/LayoutRes | layout/placement/transform/clip | font outlines and some shapes survive; no blind mask reuse |
| Worker count / cache budget | execution only | identical visual result, diagnostics and resource-limit outcome |

No “dirty everything” flag replaces this table. No “only repaint” heuristic may override a changed key. T019 tests each row by comparing cached to fresh-renderer bytes and inspecting construction counters.

## K4. Concurrent protocol

1. Coordinator enumerates requests with stable ordinals and deduplicates full keys.
2. Look up immutable hits, record ordered touches, reserve missing outputs and schedule only unique misses.
3. Jobs read inputs and write private outputs; they do not insert recursively or wait on other cache misses.
4. Coordinator joins the stage, orders results by request ordinal, publishes successes and propagates errors deterministically.
5. Eviction/admission occurs at this barrier. Downstream jobs receive Arcs directly.

One renderer has one in-flight frame. This intentionally avoids a general concurrent map/single-flight framework initially. Read-only hit snapshots can be shared with workers if measurements justify it; sharding and locks require evidence of a bottleneck. Tests use identical misses across hundreds of events, tiny capacities, retained frames, forced eviction, failed producers and worker counts 1/2/4/8.
