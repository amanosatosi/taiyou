# ADR-011: Solo-maintainer validation and explicit baseline promotion

Status: Accepted by the repository maintainer for implementation, 2026-10-18.
Authority: the owner's "Final T005 Completion (Solo-Maintainer Edition)" instruction
in PR #11's implementation session. This accepts the governance direction, not an
unreviewed implementation commit or an independent GitHub approval.
Amends ADR-005's deployment assumptions and TESTING V4; C1–C7 are unchanged.

## Decision

One human maintainer reviews reference pins, inventory/font/image identities,
tolerances, support classifications, case selection and validation infrastructure.
Ordinary GitHub Actions and ordinary branch protection are sufficient for the
current project. A second reviewer, dedicated App and separate validator are
optional future upgrades, not T005/T015 prerequisites.

Normal PR validation uses the PR base revision's accepted inventory, policy and
verification tools. Changes to policy-protected files fail without a maintainer
acknowledgment bound to the exact head, accepted base and changed-file digest.
An acknowledgment permits a proposal to be checked; it does not silently activate
new expected results or constitute final acceptance of the implementation.

Bootstrap PR #11 uses the owner-reviewed immutable snapshot
`39f6ce2fa64b231fd705862d853c8df239b54a22` until its inventory reaches main.
Bootstrap must preserve its inventory and reference lock byte-for-byte and all
regression-bearing policy fields. Only the governance metadata/infrastructure
changes authorized by this instruction are admitted without a new acknowledgment.
The final candidate still needs the maintainer's explicit acceptance.

Keep the external corpus separate, preserve every case and zero tolerance, forbid
ART_REG_* overrides and verify cached corpus bytes before/after checks. Normal CI
never generates replacement references. Future renderer execution uses read-only
assets and bounded unprivileged execution; T015 implements its comparison adapter.

## States and limitations

Inventory verified, candidate CI passed, solo-maintainer accepted and independently
trusted are distinct states. The first two do not imply either of the latter.
The current configuration does not claim independently trusted validation.
Candidate-run tamper tests prove tested checks, not resistance to malicious CI.

The maintainer/admin can deliberately change baselines, acknowledge proposals or
reconfigure protection. Agents using that same account cannot be distinguished
cryptographically from the human: agents must not post baseline acknowledgments,
self-accept protected changes or merge this PR. Required GitHub Actions checks
provide accidental-change safeguards, not a distinct trusted issuer.

## Acceptance and future upgrade

T005 can be recommended for solo acceptance after exact-commit inventory/policy,
workspace and documentation checks pass with unchanged regression data. Record
actual branch-rule evidence and any remaining setup action; never describe a
protection gate as active without evidence. The human alone accepts and merges.

An independently controlled policy repository, dedicated issuer, independent
reviewers and adversarial sandbox/issuer tests remain the stronger optional V4
upgrade. Adopting them later requires actual deployment evidence, not a label.
