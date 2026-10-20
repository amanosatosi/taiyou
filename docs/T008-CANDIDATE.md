# T008 provisional font candidate

The candidate is stacked on the separate [CI enablement proposal](T008-ENABLEMENT.md), based on main `5676edf1573cacff7dcd217243f0a94a4cc6a87a`. The owner's exception authorizes preparation and CI only. Shared-interface, dependency, unsafe/FFI, protected CI and task acceptance gates remain open, along with the historical review debt for PRs 8/9/10/12. Nothing here establishes T005's independently trusted validation.

Host bytes are copied once per verified SHA-256 digest; matching digests also compare full bytes. Face indices are checked against the native collection count, and high-bit FreeType named-instance selectors are rejected. Registration preserves order and publishes immutable snapshots through a non-cloneable revision owner. Identical bytes/index deduplicate; collections share one byte allocation. Published snapshots retain bytes after the builder, input buffers and native contexts are dropped. Public shared record constructors hold host-authored descriptors; validated runtime intake is `FontCatalogBuilder::register`, not a claim that arbitrary host metadata was authenticated.

Family matching uses ICU4X full Unicode default case folding without locale or NFC/NFKC. Exact folded family, weight distance, italic match, then registration ordinal implements C4. No installed fonts, file discovery or silent fallback is used. The new SHA-256 and casefold dependency intake remains provisional in PR 14; its exact root lock is generated on Actions. The Linux audit includes both production and isolated smoke dependencies, with versions, licenses, MSRVs and timestamped advisory queries.

Each `FontContext` owns one FreeType library. At most one temporary memory face is live, including the collection probe; a private borrow and owned byte Arc enforce destruction order. Native handles never escape and contexts are neither Send nor Sync. Immutable catalogs are Send+Sync. Isolated unsafe declarations for SFNT names and the OS/2 weight prefix match pinned public APIs; a C header ABI assertion and native tests supplement, but do not replace, independent unsafe review. Malformed-font, post-initialization failure, load/drop/reload and negative Send/Sync tests exercise cleanup. Native codes remain in errors.

Registration preflights unique retained font bytes, checked catalog/name storage and conservative native/name/copy scratch before allocation. The estimate reserves 12 times input length plus 64 KiB and existing metadata/names, covering copy/publication overlap and name decoding/folding. Four input lengths plus 64 KiB of this is a native scratch estimate; R003 must measure actual native internal allocations. These logical reservations are not a hard process RSS guarantee (C7). Tiny-budget failures publish no partial registration. Native glyph inspection has its own configurable Limits and cannot bypass those checks with a host-created blob.

CI uses the existing pinned FreeType 2.14.3/HarfBuzz 14.5.0 static recipes on Linux x64, Windows MSVC x64 and macOS ARM64, deployment target 15.0. The root native feature is built, linted and tested on each target; Linux also tests exact Rust 1.85.0. Linux builds a second FreeType with ASan/UBSan and runs the actual Rust unit/integration binaries against it with leak detection. Only the native C library is sanitizer-instrumented; no Rust sanitizer or fuzz-completeness claim is made.

Fixtures are runner-only Source Sans 3 Regular and Bold from upstream commit `ed1808970eb3c7301c9a523bee26473ba0bb62fa`, OFL-1.1. Regular retains the T002 SHA-256. Bold is verified against Git blob `d2129396eab406f5d268806c7598b78eb7bfdafb` and length 343596 from that pinned upstream tree. Exact runtime SHA-256 values are reported. A two-face OTC is generated with FontTools 4.60.2 (`recalcTimestamp=False`, `shareTables=True`) from those unchanged inputs; its digest is recorded by CI. These are catalog fixtures, not modified renderer reference outputs or libass-tests assets. FontTools is MIT, runner-only, exact wheel SHA-256 `73cf92eeda67cf6ff10c8af56fc8f4f07c1647d989a979be9e388a49be26552a`; avoiding a custom collection writer is the alternative rationale. SHA-256, ICU Unicode-3.0/data, FontTools and font notices must be included in any distribution review. FreeType license choice, component review and older deployment targets remain unresolved from T002.

T009 is not eligible until T007/T008 interfaces receive required reviews, acceptance and integration, and T007's missing-main T006 prerequisite is resolved. Handoff: preserve literal versus containing-event escape provenance; use per-call HarfBuzz OpenType buffers and font IDs/face indices, checked glyph budgets and cluster-wide fallback. Source Sans covers the current Latin/combining catalog smoke only. Arabic, Indic and CJK shaping need separately pinned licensed fixtures before making those support claims. No shaping, glyph outlines, renderer success, cache or performance claim is added here.

The initial 4.60.1 FontTools candidate failed the expanded Actions advisory audit with GHSA-768j-98cg-p3fv / PYSEC-2026-1389. The [upstream advisory](https://github.com/advisories/GHSA-768j-98cg-p3fv) identifies 4.60.2 as patched; its exact universal wheel hash was read from the official PyPI release metadata. The audit remains enabled and fails on reported advisories. No exception or suppression was added.

Catalog publication takes an explicit `&mut CatalogIngestor` owned by the renderer/session across **all** builders, rather than a fresh publisher inside each builder. Replacement catalogs therefore receive distinct monotone C1 identities; the owner is non-cloneable, revisions are not pointer/global identities, and exhaustion fails. This provisional factory argument follows the existing session identity rule and requires shared-interface review alongside the other T008 interfaces. A replacement-builder regression proves retained old snapshots and new font identities remain distinct. Folding reserves six input byte lengths plus fixed overhead to cover Unicode expansion and String capacity growth before the library call.

Review follow-up seals runtime publication. Shared `FontBlob::from_unverified`, `FontRecord::from_unverified` and `CatalogIngestor::publish_unverified` explicitly describe untrusted host descriptors. They do not yield a `ValidatedFontCatalog`. That immutable capability has private construction in the native catalog adapter; runtime `resolve_family` requires it, and no Deref/unchecked conversion exposes a bypass. The verified builder can publish it directly. Importing shared descriptors through `validate_catalog` recomputes SHA-256, validates collection faces and compares family, folded name, weight and italic against freshly derived native records. Duplicate imported identities are rejected, even if identical; ordinary builder registration still deduplicates identical bytes/index. Failed imports consume no validated output revision. Native blob inspection independently checks claimed digests before using them.

The synthetic weight/italic/order fixture retains its exact assertions inside a `cfg(test)` private-selector test. Production publication has no fixture feature, unchecked marker constructor or metadata override. Integration regressions use actual pinned fonts and reject forged hashes, conflicting bytes, duplicate IDs, family/folded-name/weight/italic spoofing and bad face indices. Compile-fail examples prove raw catalogs cannot enter runtime resolution or construct the sealed capability. The seven-package DAG, immutable shared descriptors, native ownership and normative contracts remain unchanged; these provisional helper interfaces still require review before integration or T009. The owner's acceptance of the pinned sha2/ICU selections remains subject to technical, dependency and license review.

## ASS font compatibility follow-up (2026-10-20)

This owner-requested T008 completion starts from merged PR #30, main
`4166a18fdf93927eb64a2c25c69473f6a25d881c`. It does not implement SSA V4 styles,
change system discovery/fallback, or admit LegacyXyV1 behavior. The original
candidate history above is retained.

Source inspected first: local pinterf xy-VSFilter archive labeled 3.2.0.810,
without Git metadata; equivalence to the locked upstream commit is unverified.
`src/subtitles/STS.cpp` SHA-256
`59f431e3ac36d99e2b694cdab106a530edaf7bbc8c85410f91647d3ff4578895`:
LoadFont lines 1798–1870 decodes the offset-33 six-bit attachment stream and
passes it to AddFontMemResourceEx; LoadUUEFont lines 1872–1897 accumulates
fontname-separated payloads; LOGFONTW conversion lines 3719–3731 passes the
requested family to GDI. `RTS.cpp` SHA-256
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`:
CMyFont lines 116–137 calls CreateFontIndirect. GDI performs its own font name
matching; this source does not establish an exact cross-platform alias ordering
or pixel result. Taiyou preserves its existing deterministic face-selection
ordering and uses names from that same supplied face. No Windows capture is
claimed. Invalid attachment syntax follows Taiyou's existing Strict/Recover
policy rather than reproducing the historical unchecked invalid-character path.

The pinned libass-tests metadata inspection ran in Actions run 39397731478,
job 118544523162. Frozen font bytes were not modified:

| Case | Evidence and implementation |
|---|---|
| font_nonunicode/shiftjis | ASS requests `麗流隷書`. The supplied shiftjis_Reishoreiryu.ttf (SHA-256 `cb7906711eb6bcda10ffc0888e60af8704a9460cfee6d030f3cc40a939c95fcb`) has Microsoft language 0x411 / encoding 2 name bytes `9e976d4196b766f8`, representing the requested UTF-16BE name. The old encoding-ID filter skipped it and fell back to a different name. All Microsoft name records now use their Unicode name-string interpretation. |
| faux-italics/cff | ASS requests `FansubBlock-CFF`, stored as name ID 6; family ID 1 is `Fansub Block CFF`. The supplied OTF SHA-256 is `ad6a80cad983636039b54db21c4fc484a5285268c1e8e39a216c782115e2e53e`. Full/PostScript/WWS-family aliases use the same general validated-face path as family aliases. |
| embedded-font/efont | ASS requests `wow` and carries `fontname: fansubdoge_0.otf`. The harness now permits the renderer to execute with those attachments and zero external font arguments; actual decoding and native validation remain renderer responsibilities. |
| ssa/alpha-* | SSA V4 style mapping remains unsupported. The parser reports a typed UnsupportedFeature before ASS default-style synthesis can produce a misleading FontUnavailable. No SSA mapping or replacement font is introduced. |

Aliases retain a single FontId and byte blob per face; weight distance, italic
match, then catalog ordinal remain unchanged. Localized and alternate names are
Unicode-folded without compatibility normalization. Catalog aliases are sealed
native-derived metadata, not trusted host claims. Added independently authored
Source Sans name-table fixtures are runner-only OFL derivatives; original
Regular/Bold/OTC fixtures and external corpus bytes remain unchanged. FontTools
and native dependency versions/licenses are unchanged.

Decoded attachments live in ParsedTrack with original spans. Attachment names
never reach filesystem APIs. Parse limits check cumulative decoded font bytes,
metadata and allocation overlap before appending. The rendering boundary again
checks the caller's aggregate catalog font limit and validates every face using
FreeType. Combined catalogs use a renderer-owned monotone CatalogIngestor,
without history-dependent font selection or a new cache. Callers using the raw
Track rendering API remain responsible for supplying their validated catalog;
render_parsed/CLI perform attachment registration. No new runtime dependency.

Verification and actual corpus transitions are recorded in the PR after the
focused runner checks and the final complete external comparison. Successful
font resolution alone is not an exact rendering compatibility claim.
