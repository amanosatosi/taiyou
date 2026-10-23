# ADR-015 — Transparent border collision reservation

Status: owner-authorized compatibility correction, 2026-10-23; implementation
and exact-head verification remain reviewable in the production repair PR.

The owner explicitly authorized correcting ADR-012/C4 when pinterf confirms
that transparent borders reserve collision space. This supersedes ADR-012's
invisible-effects rule only for the vertical line-border reservation below.
Policy identity: `core-collision-geometric-3`.

In the local pinterf xy-VSFilter source, RTS.cpp GetNextLine1453–1502 collects
line outline widths without inspecting paint alpha. MakeLines1536–1565 keeps
the first line's top border and last line's bottom border. AllocRect1606–1651
inflates the logical reservation by those borders before collision placement,
then removes them when returning the text position.
The inspected local RTS.cpp SHA-256 is
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`;
the archive name is not a Git revision or authenticated xy output provenance.

Union the existing geometric footprint with the logical event bounds expanded
upward by the first line's maximum border Y and downward by the last line's
maximum border Y. Include transparent paint, compacted whitespace and opaque
box styles. A genuinely empty mandatory-break line has zero border reservation.
Interior-line borders do not become first/last-line padding. Use the existing
SBAS/device mapping and nearest eighth-pixel reservation in that space, then
convert back to script coordinates. Do not alter local metrics or glyph anchors.

Keep the existing visible geometric effect hulls, horizontal footprint, active
set ordering, explicit-position/origin exclusions and stateless interval solver.
This narrow correction does not claim complete pinterf allocator or pixel parity.
Protected external fixtures, arithmetic and acceptance remain unchanged.
