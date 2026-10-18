# Solo-maintainer validation setup

[ADR-011](../docs/decisions/011-solo-maintainer-validation.md) is the owner's
explicit governance decision. A second human, separate validator and dedicated
App are not required. No agent may fabricate acceptance or merge PR #11.

## Recommended GitHub protection

At [Settings / Rules / Rulesets](https://github.com/amanosatosi/taiyou/settings/rules)
create an Active branch ruleset targeting main (or use Settings / Branches and an
equivalent classic rule):

- Require a pull request before merging, with **zero required approvals**.
- Require branches up to date and these checks: **Corpus policy gate**,
  **Fast gate**, **Platform gate**, **Documentation structure**.
- Require conversation resolution; block force pushes and deletions.
- Apply checks to administrators where supported; do not add agent bypasses.
- Do not require code-owner/last-pusher/second-person approval for this solo setup.

Select GitHub Actions (App ID 15368) as source where the UI offers it. This is
ordinary CI, not a dedicated independent issuer. Corpus policy gate runs on every
PR without path filtering. Fast/Platform gates aggregate selected checks and
handle documented skips; do not require individual conditional native jobs.

After saving, inspect branches/main, rules/branches/main and the rule's required
contexts/review count. Confirm the sole maintainer can merge a passing PR without
another user's review. Record actual evidence; a proposed rule or this guide is
not proof of enforcement. An administrator can deliberately change the rule.

## Human acknowledgment of policy changes

Ordinary implementation PRs use the exact PR-base inventory/policy/verifier.
No baseline acknowledgment is needed when policy-protected files are unchanged.
The baseline guard prints changed protected paths and a change digest. For an
intentional reference/policy/infrastructure proposal, the human maintainer reviews
the diff and posts exactly the printed comment on that PR:

```text
TAIYOU_BASELINE_ACK <head SHA> <accepted base SHA> <change digest>
```

Then rerun the failed Actions job. Only the repository owner's exact comment is
recognized; any new head/base/digest invalidates it. Agents must not post this
comment using delegated owner credentials. GitHub cannot distinguish such an
agent from the human, so this remains a governance rule, not independent security.

The acknowledgment allows proposal checking but does not activate new comparison
data. CI continues using the accepted base snapshot. The human must separately
accept the exact final candidate and merge it before it becomes a future baseline.
PR #11 has the narrow owner-authorized ADR-011 bootstrap exception: reviewed
snapshot 39f6ce2fa64b231fd705862d853c8df239b54a22, identical inventory/reference
lock and regression policy, governance/infrastructure changes only.

## Reference, tolerance and capability proposals

1. **New external revision:** use a focused upgrade PR with exact upstream commit,
   original Git blobs, all source/font/image SHA-256 identities, symlink targets,
   provenance/licenses and complete old/new case accounting. Explicit runner-only
   proposal generation is separate from ordinary verification. Verify the proposed
   new corpus in an explicitly reviewed Actions job; acknowledgment alone does
   not verify it. Never overwrite originals or regenerate PNGs to hide a failure.
2. **Tolerance change:** present per-case/profile numeric bounds, alpha constraints,
   reason and before/after visual evidence. Zero tolerance is the current default;
   the current policy checker rejects all tolerance exceptions. A future exception
   needs reviewed implementation as well as policy acknowledgment. No blanket
   widening, skipped failures or candidate-side replacement threshold is allowed.
3. **Support promotion/demotion:** name every affected case/profile and feature,
   show exact-commit execution evidence and retain all unsupported/failing cases
   in the denominator. Changing a label is not successful rendering. Historical
   T014 reason/protection strings in the frozen inventory remain historical;
   this session changes no classification.

Suspicious changes include deleted expected files/cases, changed hashes without
upstream provenance, fewer result records, duplicate IDs, unexplained unsupported
labels, empty selectors, relaxed tolerances, ART_REG_* variables, removed checks
or a workflow that never runs on unrelated PRs. Investigate the defect; do not
update expected results merely to make CI pass.

## Honest boundaries and optional upgrade

Inventory verified, candidate CI passed, solo-maintainer accepted and independently
trusted are separate states. Human acceptance is not created by a test, comment
from an agent or this document. Current ordinary Actions are not independently
trusted. Read-only file modes and pre/post hashes catch accidents, but candidate
code running as the same OS user can change permissions or its CI. No malicious
candidate containment/credential-isolation claim is made.

The [previous stronger setup](https://github.com/amanosatosi/taiyou/blob/39f6ce2fa64b231fd705862d853c8df239b54a22/validation/MAINTAINER-SETUP.md)
and preserved protection observations describe the optional independent upgrade.
They are historical guidance, not prerequisites for T015. T015 starts only after
the human accepts and integrates T005; no comparison engine is implemented here.
