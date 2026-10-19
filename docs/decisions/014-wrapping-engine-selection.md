# ADR-014 — Event-level wrapping-engine selection

Status: **owner-directed implementation; final integration/review pending**.
The owner explicitly requested working source-derived mode0 automatic wrapping,
rather than a selector-only feature. The human merged PR29 at
`6cfa33ddb38c714a43a096abf53096f9f9aff69c`, the integration base.

## Selection and scope

The Taiyou extension `\warpmode` selects one immutable engine for the entire
event: absent or0 selects pinterf xy-style word wrapping;1 selects the separately
gated ADR-013 Unicode algorithm. The last valid standalone parsed selector wins,
including a tag after text. Style resets, breaks and paint changes do not reset
it. Nested children of unsupported operations never execute. Literal text
cannot select an engine.

The complete parameter is ASCII0 or1 with optional ASCII space/tab. Invalid
parameters fail strict parsing; recovery diagnoses and retains the earlier
selection. This finite grammar is a Taiyou extension, not an xy tag.
Every operation retains its original source span; default0 has no invented span.

The independent q axis starts with Script Info WrapStyle. Supported q operations
follow the local pinterf decimal-prefix conversion: an empty or out-of-range
parameter restores the script default; a nonempty nondigit prefix converts to0.
Parenthesized nonempty parameters precede a command suffix. Style reset does not
reset q. Decode each text fragment under the q state at that source position:
`\N` is mandatory; `\n` is a space in0/1/3 and a newline in2; `\h` is NBSP.
The final q state selects the event's line policy. Thus a later q does not
retroactively reinterpret an already decoded soft break.

## Mode0 implementation

Use ASCII-space/nonspace groups, not Unicode break opportunities. q1 uses the
available width; q2 invokes the unchanged explicit-break layout; q0 derives the
remaining hard-line width target and q3 applies the additional final-word
adjustment documented in the source below. Accept an overwide first group,
consume only whole groups, and trim leading/trailing ASCII-space words after
fitting. Preserve consumed source ranges even when their glyphs are trimmed.

Available width is PlayResX minus inherited event left/right margins, using the
original event style for inheritance. Border widths are not subtracted.
HarfBuzz supplies word advances with existing fallback, cluster ownership and
style scaling. The policy uses checked eighth-script-unit widths derived from
those advances. Words retain their own shaping context; no word is split at an
automatic line boundary. Visual run ordering follows the resulting lines.
Core empty-line metrics and q2 behavior remain authoritative.

This implements the requested wrapping policy with Taiyou's font backend.
Different font metrics and shaping can change the resulting line breaks.
It is not a claim of GDI metric equivalence, authenticated xy capture agreement,
pixel compatibility or admission of any LegacyXyV1 capability.

Mode1 automatic wrapping continues to return UnsupportedFeature in both policies,
without mode0 fallback or partial output, until ADR-013's conformance, native-work,
preflight and separately reviewed C7 limits are satisfied. q2 uses the shared
hard-break path for both selectors. No new limit defaults or dependency are added.

## Local source evidence

Read-only local pinterf pfmod3.2.0.810-labelled source, from LOCAL_REFERENCES:
RTS.cpp SHA-256
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`;
STS.cpp SHA-256
`59f431e3ac36d99e2b694cdab106a530edaf7bbc8c85410f91647d3ff4578895`.
Archive equivalence to the locked upstream commit remains unverified.
No GPL code is copied into Taiyou and no source tree is modified.

| RTS.cpp function / lines | Observed policy |
|---|---|
| CText::GetTextInfo 698–724 | Width conversion and eighth-pixel storage |
| CLine::Compact 1171–1210 | Trim edge whitespace after fitting |
| CSubtitle::GetFullLineWidth 1409–1419 | Width through the next hard break |
| CSubtitle::GetWrapWidth 1421–1450 | q0 target, q1 width, q2 unbounded, q3 adjustment |
| CSubtitle::GetNextLine 1453–1503 | Whitespace groups, overwide first group |
| CSubtitle::MakeLines 1537–1563 | Available width and margins |
| CRenderedTextSubtitle::ParseString 1966–2020 | N/n/h decoding and space grouping |
| CRenderedTextSubtitle::ParseSSATag 2047–2215, 2612–2625 | Parameter ordering, q conversion, style-only reset |
| CRenderedTextSubtitle::GetSubtitle 2872–2991 | Script q initialization, source-order evaluation, margins |

Source observations establish policy, not visual capture evidence. R001 proposals,
authenticated xy capture requirements and the independent protected libass suite
are unchanged.

## Verification and integration

Executable tests are `crates/taiyou-cli/tests/automatic_wrapping.rs`,
`crates/taiyou-cli/tests/warpmode.rs` and the private xy_wrap unit tests.
They cover q decisions, escapes, overwide words, margins, source ordering,
malformed q conversion, selector/reset behavior and fresh/repeated/seek equality.
Native execution occurs only on Actions. Test results are reported separately
from source findings; no proposed fixture becomes a trusted reference image.

The renderer adapter uses PR29's accepted collision/backend stages, preserving
F1, complete-active-set placement, adaptive retention and diagnostics. Run the
full required validation on the integrated candidate.
This is not completion of mode1 or full T017.
