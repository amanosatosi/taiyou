# Compatibility policy

Implement established behavior → compare → fix discrepancies → verify → record
findings. [Compatibility notes](compatibility-notes.md) describe tested findings
and remaining gaps. Ordinary source-backed corrections require no new behavioral
contract, research admission or ADR approval. Obsolete CoreV1 assumptions yield
to verified historical behavior. Safety, public APIs, resource accounting,
determinism, licensing, protected comparisons and human merge approval remain
engineering safeguards.

## B1. Authority and profiles

The main xy authority is exclusively **pinterf/xy-VSFilter**. Read the supplied [local pinterf source](C:/taiyou-time/refrences/xy-VSFilter-3.2.0.810/README.md) first; [the local reference map](LOCAL_REFERENCES.md) distinguishes its archive identity from the earlier locked upstream revision. Source inspection can establish semantics directly. Exact output claims require authenticated captures with recorded Windows build/font/locale/render configuration; a local archive name does not establish that provenance. Guliverkli2 is separately named historical evidence. Neither is interchangeable with libass or another VSFilter fork.

Order of decision: verified pinterf/xy-VSFilter source behavior for ordinary ASS/SSA semantics; explicit Taiyou extension contracts only where intentionally different; narrow targeted investigation only when the reference source is genuinely ambiguous. libass is an independent cross-platform comparison oracle and engineering source, not a fallback authority for xy behavior. Format manuals clarify syntax but cannot prove historical raster pixels. Mangetsu/libassmod is future extension evidence only.

`CoreV1` is Taiyou's current implementation profile. A Core behavior proven incompatible with ordinary ASS semantics must be corrected to historical xy behavior unless an intentional Taiyou extension explicitly owns the difference. `LegacyXyV1` remains representable for compatibility bookkeeping, but no research-task admission is required before implementing source-established behavior. Unsupported rules produce diagnostics or strict failure.

## B2. Documented disagreements

The upstream [differences document](https://github.com/libass/libass/wiki/Differences-between-Libass-and-VSFilters) reports the following; it is contextual evidence, **not a new comparison run of our pinned builds**:

| Area | Reported disagreement | Taiyou disposition |
|---|---|---|
| Borders under transforms | VSFilter preserves border widths through transforms; libass differs | Follow verified xy transform/stroke behavior when implemented |
| Automatic wrapping | libass balances lines differently from VSFilter | Follow verified xy wrapping for standard mode; `\warpmode1` is the separate Taiyou Unicode extension |
| Missing SBAS header | libass has FFmpeg-generated-file exceptions | Explicit profile parsing rule, no accidental heuristic |
| Style Encoding / `\fe` | Font-selection constraints differ | T026; no locale-derived guesses |
| Windows font/bidi environment | Same VSFilter can vary by Windows/region | Capture environment; bound claims to it |
| HTML-like tags | VSFilter parses some that libass does not | Unsupported initially, visible in inventory |
| Banner/Scroll fadeaway fields | libass omits some legacy effect fields | Deferred unsupported cases, not silently passed |
| Nested parentheses | Historical variants disagree | Tokenization fixtures per profile, no universal recursive parser |

This table deliberately distinguishes contextual reports from Taiyou's own source-verified behavior and pixel evidence. Source inspection can establish semantics; pixel identity still requires comparison evidence.

## B3. Review and support registry

The protected inventory records comparison coverage and support classifications.
Descriptive findings belong in compatibility notes with source/test evidence,
implementation status and remaining discrepancies. A missing note is not proof
of support or a reason to block a verified correction. Internal passing tests
alone do not establish historical compatibility.

Each test/profile pair has exactly one result: `PASS_EXACT`, `PASS_APPROVED_TOLERANCE`, `EXPECTED_DIFFERENCE` (approved issue+scope), `UNSUPPORTED` (feature ID), `REGRESSION`, `CRASH`, or `INFRA_ERROR` (missing font/oracle/asset). Infrastructure failures never count as unsupported. Expected differences are not counted as exact compatibility. A formerly passing case becomes a regression even if an agent now thinks its feature unsupported.

An intentional difference proposal includes original script/fonts, captured reference provenance, before/after Taiyou output and numerical diffs, source reasoning, affected profile and all impacted cases. Architect recommends; repository maintainer approves separately from the implementation author. The policy update merges separately or as an explicitly protected review, with permanent audit trail. Never regenerate an external image to make Taiyou look correct.

## B4. Unresolved boundaries

The current owner-directed batch implements LayoutRes, source-consumed unknown
commands (including board1's b-prefix recovery), vertical @ font requests and
standard BorderStyle=3 boxes. [Source paths and focused native witnesses](T018-CANDIDATE.md#layout-unknown-commands-vertical-fonts-and-opaque-boxes)
record the implementation boundary. These are working renderer features;
unresolved font/raster pixel parity is reported separately from execution.

GDI versus FreeType outline/span raster differences, overlapping translucent runs, ligatures crossing paint boundaries, fractional rotation, legacy YCbCr conversion, vertical fonts, unusual codepages and some malformed-tag precedence remain unresolved. R004 remains a narrow investigation only where transform source behavior is genuinely ambiguous. V1 output is encoded RGB overlay; historical video-space color conversion is a host adapter/research task and must remain an explicit compatibility limitation. `.asa` and exact Mangetsu identity/specification need authoritative material before syntax contracts.
