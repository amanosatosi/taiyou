# ADR-009: Core reset-token disambiguation

Status: **Accepted by explicit repository-owner architectural decision (choice B), 2026-10-17.** The owner selected exact declared-style matching in the review follow-up. This records owner-level acceptance, not independent third-party review or protected integration approval. CONTRACTS.md is updated in this separate policy PR; implementation remains separately reviewable in PR 8.

## Concrete ambiguity

C2 retains unknown operations, while C3 permits `\rName`. Names are preserved without a reserved namespace. The same bytes `\rnd` can therefore denote reset to style `nd` or an unsupported `rnd` operation. PR 8 currently recognizes every `r` prefix as ResetStyle, including `\rnd30`; a future extension can silently change style state. Rejecting all such prefixes would instead break legitimate named resets. Neither precedence is specified by the current contract.

Local source observation: `C:\taiyou-time\refrences\mangetsu\docs\rnd-tags.md` lines 3–9 documents rnd/rndx/rndy/rndz/rnds as extension syntax. This is an unverified local archive observation, not authenticated upstream revision evidence or a LegacyXyV1 rule. No reference implementation was copied and no renderer was run. The main xy authority remains pinterf as specified in COMPATIBILITY.md.

## Accepted intended Core behavior (choice B)

1. Bare `\r` is always ResetStyle(None).
2. After the existing structural whitespace trimming, `\rName` is ResetStyle(Some(Name)) only when Name exactly matches a retained declared style name. Match case-sensitively, using the same post-section name resolution as event styles; forward definitions and last-definition-wins remain valid. The synthesized Default also qualifies.
3. Other `r`-prefixed operations remain Unsupported with their full original span and a diagnostic. Do not guess reset-to-Default or execute a future extension. Preserve the existing unknown-name extraction; no extension is enabled by this rule.
4. If a style named `nd` exists, `\rnd` is its named reset. If a style named `nd30` exists, `\rnd30` is its reset. Style existence deliberately resolves the unavoidable collision; a future syntax addition must review that collision and its profile revision. Do not grow an ad hoc list of reserved future tag names.

| Input | Declared style names | Core token |
|---|---|---|
| `\r` | any | reset current event style |
| `\rNamed` | Named, Default | named reset Named |
| `\rnd` | nd, Default | named reset nd |
| `\rnd` | Default only | Unsupported rnd |
| `\rnd30` | Default only | Unsupported rnd30 |
| `\rndx1` | ndx1, Default | named reset ndx1 |
| `\rMissing` | Default only | Unsupported rMissing, **not** implicit Default reset |
| `\r日本語 Style` | 日本語 Style, Default | exact named reset |

This changes handling of unresolved named-reset tokens in PR 8 under the owner's explicit decision. It is not a Legacy compatibility claim or support-registry promotion.

## Alternative A: lexical reset prefix always wins

Keep current PR 8 behavior: every `r` prefix consumes the remainder as a style name, including unknown names and extension-looking names. This preserves unrestricted lexical named resets without consulting style definitions, but Core must explicitly classify `\rnd30` as reset to nd30 rather than unsupported. Future tag admission then requires a reviewed precedence/profile change. It does not solve accidental style-state changes from extension-looking input.

## Rejected shortcut

Reserve only known rnd spellings before matching r. This is incomplete for future names, introduces an undocumented namespace and can steal resets to legitimate colliding style names. Rejecting all alphabetic suffixes also breaks C3 named resets.

## Migration and review gates

The owner decision authorizes PR 8 to tokenize against the final post-section style-name table and add executable regressions for every row above, forward style definitions, duplicate definitions, strict/tolerant syntax diagnostics and retained unknown spans/order. Keep legacy nesting/recovery research in R001 and evaluation in T007. Integrate this policy PR before PR 8 only after the applicable protected review requirements are met. No owner acceptance is represented as independent review.

Independent PR 8 repairs may proceed now: enforce the existing text-byte-derived operation cap for every host-created TextOp variant before iteration/publication, and document that C2's unit escape variants carry event-level rather than individual source spans. Those repairs do not resolve or approve the reset grammar decision.
