# Compatibility policy

## B1. Authority and profiles

The main xy authority is exclusively **pinterf/xy-VSFilter**, with a recorded Windows build/font/locale/render configuration. Read the supplied [local pinterf source](C:/taiyou-time/refrences/xy-VSFilter-3.2.0.810/README.md) first; [the local reference map](LOCAL_REFERENCES.md) distinguishes its archive identity from the earlier locked upstream revision. A local archive must not silently replace that oracle without revision verification/review. Source behavior plus reproducible captures adjudicate rules. Guliverkli2 source is now locally available as separately named historical evidence; an exact reference build/captures are still required. Neither is interchangeable with libass or another VSFilter fork.

Order of decision: explicit Taiyou contract/profile rule with approved evidence; pinned xy behavior for an unresolved LegacyXyV1 rule; a separate original-VSFilter profile proposal if history disagrees; otherwise bounded research. libass is an independent cross-platform comparison oracle and engineering source, not an automatic fallback for missing xy evidence. Format manuals clarify syntax, but cannot prove historical raster pixels. Mangetsu/libassmod is future extension evidence only.

`CoreV1` has the exact development rules in [contracts](CONTRACTS.md). It enables the vertical slice without pretending to emulate undocumented quirks. `LegacyXyV1` is the release-oriented profile, admitted family by family by R001/R002/R004. Unsupported rules produce diagnostics or strict failure. There is no automatic profile switching based on content and no arbitrary collection of user-visible quirk toggles in v1. Semantic changes bump a profile policy revision and retain fixtures.

## B2. Documented disagreements

The upstream [differences document](https://github.com/libass/libass/wiki/Differences-between-Libass-and-VSFilters) reports the following; it is contextual evidence, **not a new comparison run of our pinned builds**:

| Area | Reported disagreement | Taiyou disposition |
|---|---|---|
| Borders under transforms | VSFilter preserves border widths through transforms; libass differs | Legacy stroke after transform; R002 verifies exact xy behavior |
| Automatic wrapping | libass balances lines differently from VSFilter | R001 freezes xy wrap fixtures; Core starts with hard breaks |
| Missing SBAS header | libass has FFmpeg-generated-file exceptions | Explicit profile parsing rule, no accidental heuristic |
| Style Encoding / `\fe` | Font-selection constraints differ | R001/T026; no locale-derived guesses |
| Windows font/bidi environment | Same VSFilter can vary by Windows/region | Capture environment; bound claims to it |
| HTML-like tags | VSFilter parses some that libass does not | Unsupported initially, visible in inventory |
| Banner/Scroll fadeaway fields | libass omits some legacy effect fields | Deferred unsupported cases, not silently passed |
| Nested parentheses | Historical variants disagree | Tokenization fixtures per profile, no universal recursive parser |

This table deliberately distinguishes reported differences from verified pinned-output differences. The initial verified-difference registry is empty because no oracle was run. R001 adds source locations and captures before promoting any row to verified.

## B3. Review and support registry

Every capability record has: ID, profile+policy revision, syntax/parameter limits, owning task, supporting fixture IDs, oracle and build digest, evidence status (`contract-only`, `source-observed`, `captured`, `approved`), support state and reviewer decision reference. Missing entries are unsupported, not assumed correct.

Each test/profile pair has exactly one result: `PASS_EXACT`, `PASS_APPROVED_TOLERANCE`, `EXPECTED_DIFFERENCE` (approved issue+scope), `UNSUPPORTED` (feature ID), `REGRESSION`, `CRASH`, or `INFRA_ERROR` (missing font/oracle/asset). Infrastructure failures never count as unsupported. Expected differences are not counted as exact compatibility. A formerly passing case becomes a regression even if an agent now thinks its feature unsupported.

An intentional difference proposal includes original script/fonts, captured reference provenance, before/after Taiyou output and numerical diffs, source reasoning, affected profile and all impacted cases. Architect recommends; repository maintainer approves separately from the implementation author. The policy update merges separately or as an explicitly protected review, with permanent audit trail. Never regenerate an external image to make Taiyou look correct.

## B4. Unresolved boundaries

GDI versus FreeType outlines/metrics, blur arithmetic, overlapping translucent runs, ligatures crossing paint boundaries, fractional rotation, legacy YCbCr conversion, vertical fonts, unusual codepages and malformed-tag precedence are not solved by this blueprint. R001/R002/R004 are bounded gates. V1 output is encoded RGB overlay; historical video-space color conversion is a host adapter/research task and must remain an explicit compatibility limitation. `.asa` and exact Mangetsu identity/specification need authoritative material before syntax contracts.
