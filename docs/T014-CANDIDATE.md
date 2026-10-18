# T014 single-worker CoreV1 candidate

Integration base: merged T012 c8bd6968c893027a2fd439d670028b6861d64ff4.
Keep unmerged for shared-interface, dependency and CI review. No independently
trusted validation gate, owner acceptance, external parity or performance claim.

## Pipeline and capabilities

C7 Renderer accepts Arc<Track>, Arc<ValidatedFontCatalog> and RenderRequest.
render_parsed additionally preserves bounded ingestion diagnostics in Frame and
honors strict policy for recover-mode malformed or unsupported parser records.
The typed-Track entry point leaves ingestion diagnostics to its host owner.
Only CoreV1, one worker and disabled cache are admitted. Contexts are created
inside a render call and destroyed there, so Renderer can move while idle without
migrating native handles. Completed Frame pixels retain the existing origin
OutputBudget lease. Adding diagnostics does not copy pixels or detach that lease.

Active events use start <= now < end and sort by (layer, source EventId). Fill
DrawOrder is (layer, EventId, 2, group ordinal); no duplicate keys. T007–T013 stage
implementations are reused. Device mapping occurs once in T011, and T012 mask
bounds are already absolute, so DrawOp offset is zero. Composition clips to full
viewport; source mask support is retained without premature clipping.

Natural script anchors use x=L, (L+PlayResX-R)/2, PlayResX-R for left/center/right;
y=PlayResY-V, PlayResY/2, V for bottom/middle/top. Event margins inherit existing
C2 zero-value rules. Explicit pos bypasses margins. T010 already offsets each
logical line relative to that anchor. Content mapping uses independent x/y ratios.
No automatic wrapping/collision displacement is implemented (T017).

Static text, colors/alpha, named resets, scales, alignment/pos, hard breaks and
font fallback use existing typed operations. Strict mode fails unsupported tags
and style border/shadow/decorations with event/source context. Tolerant mode uses
C3's diagnostic fallback: unsupported operations leave prior state unchanged;
style effects/decorations are omitted with an explicit fill-only diagnostic.
This fallback is reported as unsupported, never a supported fixture or parity.
CLI parsing is UTF-8 and strict for malformed known records. Its default rendering
policy is strict; --unsupported tolerant explicitly opts into diagnostic fallback.
Default ASS border/shadow values therefore need explicit zero values for strict
fill-only inputs. No new tag families, effects, transforms or support registry
promotions are introduced.
LayoutRes and legacy YCbCr-emulation headers on active input are reported as
unsupported; Core uses its contracted PlayRes mapping and encoded RGB.

## Reservations

The coordinator preflights output dimensions/retained origin bytes, reserves
composition publication and worst-case bounded diagnostic storage, and processes
events serially. Stage limits subtract retained masks, DrawOps and current-event
evaluated/shaped/layout/geometry ownership. All previous groups in an event remain
charged. Frame path segments accumulate across events. Native raster scratch
retains T012's conservative reservation. Taiyou/native scratch estimates are not
a hard process RSS bound; R003 measurement remains separate. No persistent caches.

## PNG dependency and export

png =0.18.1, default-features=false, optional, owned only by taiyou-cli/native.
License MIT OR Apache-2.0; upstream image-rs/image-png. Purpose: bounded lossless
PNG encoding and test decoding. Upstream Cargo.toml declares Rust 1.73; exact
1.85.0 verification and the full transitive lock/advisory audit run on Actions.
See https://github.com/image-rs/image-png/blob/master/Cargo.toml and the locked
registry checksums. Alternative image adds unrelated formats; libpng adds another
native dependency; a handwritten encoder would duplicate an audited codec.
No new native dependency or unsafe boundary is introduced. Admission checker
permits only this exact optional dependency/owner/features and tests tampering.

Raw RGBA is premultiplied, tightly packed, top-down encoded-domain bytes. PNG
conversion copies pixels and applies a==0 ? 0 : min(255,(c*255+a/2)/a) to each RGB
channel, preserving alpha. No gamma/color-space transform or internal mutation.
Conversion/codec scratch has a conservative checked reservation. The additional
taiyou executable shares the existing taiyou-cli entry function; --version and
the default Cargo executable remain compatible. Explicit font order is preserved;
The rendering binary is built with --features native against the pinned recipe;
default pure-Rust builds fail rendering explicitly with UnsupportedFeature.
each --font currently registers collection face zero. Library catalogs retain
their existing TTC/OTC face-index selection. No implicit system discovery.

## Evidence

tests/taiyou/core-slice/first.ass is authored for this task and uses the existing
pinned, OFL-1.1 Source Sans 3 fixture. Actual Taiyou CLI PNG/raw outputs, font/native
source pins, source/output hashes, time, dimensions, diagnostics and focused test
logs are archived for seven days using the existing pinned upload action.
The CLI PNG is independently decoded with bounded stdlib verification and exact
C6 expected straight pixels; it is never produced by that verifier.

renderer-reference.c independently calculates single-H metrics, nine anchors,
negative placement and Q6 contours using pinned FreeType, then rasterizes via its
bitmap API. Exact RGBA tests use these coverages and independently calculated C6
arithmetic. This is candidate-side reference evidence, not a promoted golden or
T005 protection. Existing fixtures/pins/tolerances remain unchanged. Full platform,
MSRV, native sanitizer and existing regression gates remain required at final SHA.
