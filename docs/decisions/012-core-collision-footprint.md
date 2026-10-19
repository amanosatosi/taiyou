# ADR-012 — Core geometric collision footprint

Status: owner-approved for CoreV1 implementation, 2026-10-19. Authority: the
human review reply on T017 PR #29 work: “F1 is approved for CoreV1. Proceed with
the separate ADR and C4 amendment.” Final technical review and ADR-011 exact-SHA
baseline acknowledgment remain separate acceptance gates.

Amends C4 only for CoreV1. Policy identity: `core-collision-geometric-2`.
No LegacyXyV1 rule or compatibility claim is authorized.

## Decision — Core geometric effect footprint

Keep the existing local logical/ink union as the base reservation (including
whitespace and full empty-line metrics). For every positioned glyph with known
nonempty ink, transform the four corners of its font ink rectangle through the
same size, run scale, baseline, rotation/origin and script-to-device affine map
as C5. Take the device-space axis-aligned hull. Unknown ink is an error, never a
font-size estimate. This is a conservative geometric hull, not a bitmap hull.

For each compatible C5 group, union these glyph hulls to obtain I. Let r be the
actual Core isotropic border radius in device pixels. It is border size when
SBAS=false, or border size times the common device/script ratio when SBAS=true.
Use the same nearest/ties-away Q6 radius as the stroker, divided by 64; r=0 has
no expansion. Let E be I expanded by r on all sides (round widening's AABB).
Union into the base reservation:

- I when primary opacity is nonzero;
- E when a positive Outline border and outline opacity are nonzero;
- E translated by the actual Q6 shadow offset when a border exists and shadow
  opacity is nonzero, otherwise I translated by that offset for a visible shadow.

Transparent outline paint does not suppress E as the source of a visible shadow.
Transparent shadow paint adds nothing and performs no offset quantization.
Zero-offset visible shadow still contributes its source support. An empty ink
hull contributes no effect support. Taking E rather than the E-F ring's holes is
an intentional conservative rectangle policy. Invisible effects add no padding;
the base logical reservation remains even for entirely transparent text.

Convert device hulls back to script coordinates before union with the base box.
Shadow offsets scale independently on x/y only when SBAS=true. With SBAS=false,
device widths/offsets convert back by the respective mapping ratios. SBAS=true
isotropic borders with unequal x/y ratios remain unsupported exactly as C5;
this proposal does not admit anisotropic stroke. Unequal mapping with border0,
or SBAS=false borders, is defined. Rotation precedes device-space expansion and
shadow translation; border width does not rotate/scale with glyph contours.
Apply per-run effects before the event union. Negative/offscreen bounds are kept.

Coordinates stay checked finite f64. Only the already specified effect Q6
parameter quantization is used; do not floor/ceil boxes to bitmap pixels, add an
epsilon, or invent antialias padding. The footprint moves continuously with the
event anchor and C4 chooses minimum displacement in script space. This policy
separates geometric support: antialias coverage at touching geometric edges may
share a boundary pixel. It does not promise zero shared raster pixels or xy
pixel parity. If strict pixel-support separation is desired, that is a different
policy requiring a phase-aware rounding/placement decision before implementation.

Exact synthetic fixtures (unit mapping, top alignment, natural anchor 0, base
and I=[0,0,10,10], all mentioned paints visible unless stated):

| Input | Expected local footprint / second anchor Y |
|---|---|
| Two events, bord2 each | [-2,-2,12,12] each; second Y=14 |
| Two events, bord5 each | [-5,-5,15,15] each; second Y=20 |
| First bord5, second bord2 | second Y=17 |
| bord2; shad=(3.5,-2.25) | [-2,-4.25,15.5,12] |
| Same with shadow alpha0 | [-2,-2,12,12] |
| bord2, outline alpha0, shadow alpha0 | base [0,0,10,10] |
| bord2, outline alpha0, visible shadow=(3.5,-2.25) | union base with [1.5,-4.25,15.5,9.75] |
| SBAS=false, sx=2,sy=3, bord2 | script padding=(1,2/3) |
| SBAS=true, sx=sy=2, bord2 | script padding=(2,2) |
| SBAS=true, sx=2,sy=3, bord2 | existing UnsupportedFeature |
| SBAS=false shadow1, sx=2,sy=3 | script shadow=(1/2,1/3) |
| SBAS=true shadow1, sx=2,sy=3 | script shadow=(1,1) |

Also verify 90-degree rotated I=[0,0,10,4] maps to [0,-10,4,0]
before device border expansion; multiple differing run effects union separately;
fractional radii use actual Q6 values; zero-size/space-only ink adds no padding.
Pinned-font ASS integration compares displacement changes with borders, hidden
shadows, rotated styles, explicit positions, scaling and direct/sequential seeks.
No existing expected asset is changed to establish these fixtures.


## Fixed-origin boundary

The current Core evaluator has no supported explicit rotation origin. Its
rotation origin follows the alignment anchor, so this footprint translates with
collision displacement without changing shape. A future fixed explicit origin
would rotate displacement relative to that origin: the footprint could change
with the proposed anchor. That feature must revise/revalidate placement before
admission; it must not reuse this translation-only footprint or silently bypass
collision. Until then, a non-None explicit origin for an eligible footprint is
UnsupportedFeature. Explicitly positioned events remain outside collision.

## Consequences

Local layout remains paint-independent. Footprint is a separate placement-stage
value depending on layout, per-run geometry/effects/paint visibility, mapping and
this policy identity. Future collision keys include these dependencies and the
complete eligible active set. Native raster, masks, pass order and clips are
unchanged. This is a conservative geometric policy, not pixel-support separation.
