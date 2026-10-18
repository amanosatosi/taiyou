# T005 maintainer checkpoint

T005 is **blocked**, not operational or accepted. The refreshed inventory and
primitive tests can pass without providing V4 protection. No repository settings
were changed by this session. See [observed evidence](protection-evidence.json).
The connected account has repository admin permission; the blocker is not a
claimed lack of admin access. PR #11 has no submitted GitHub reviews, named
independent validator owner, approved trusted revision or established gate issuer.

## Deployment choice for this repository

Taiyou is a public, personally owned repository. Public-repository branch
protection/rulesets are available without assuming a paid plan. The account's
actual subscription was not exposed by the inspected repository metadata.
Organization-level required-workflow rules are not established for this personal
repository. Requiring a check name from GitHub Actions (App ID 15368) cannot
distinguish a trusted workflow from a candidate job using that name.

Use a separately protected, maintainer-owned validator repository and a dedicated
GitHub App issuing **`Taiyou trusted validation / corpus-gate`** on the exact Taiyou
candidate SHA. This is a deployment proposal awaiting real repository/App/reviewer
identities, not a functioning service or an invented issuer. Do not use the
generic GitHub Actions app as the required source for this trusted context.

## Exact setup and verification

1. At [Taiyou Settings / Collaborators](https://github.com/amanosatosi/taiyou/settings/access),
   appoint an independent reviewer with Write or higher permission. Supply their
   actual GitHub login and a maintainer-owned validator repository URL. Reviewer
   identities must differ from the PR author and latest pusher where required.
2. At [Settings / Rules / Rulesets](https://github.com/amanosatosi/taiyou/settings/rules),
   create an **Active** branch ruleset targeting `main`: require pull requests,
   at least one independent approval, stale-review dismissal, approval of the
   latest push by someone other than its pusher, code-owner reviews, conversation
   resolution, no force pushes/deletions and no bypass actors, including agents
   or administrators. Apply equivalent rules to the validator's default/trusted
   branches before approving its first immutable deployment revision.
3. The independent maintainer must review and publish actual CODEOWNERS entries
   for `validation/**`, reference locks/assets, support/tolerance registries,
   `.github/**`, selectors/check scripts, build/security recipes and normative
   architecture/validation documents. Protect CODEOWNERS itself. A proposed file
   on an unprotected branch is not evidence that these rules are enforced.
4. At [Developer Settings / GitHub Apps](https://github.com/settings/apps), create
   or identify the dedicated validator App. Record its real App ID, slug and
   installation ID. Grant Checks: write and Metadata: read on Taiyou; grant only
   additional read permissions needed for candidate acquisition. Do not grant
   Contents: write or give its credentials to candidate jobs. Keep credentials
   only in the separately protected validator deployment/reporting environment.
5. Independently review/deploy the trusted build/validation recipe. It checks out
   the candidate SHA in a separate unprivileged build job with read-only token,
   no secrets and no trusted cache writes. Trusted policy/harness/inventory/assets
   come from the approved validator revision, never the candidate branch or
   candidate-supplied snapshot hashes. Verify artifact provenance and binary SHA.
6. Before enabling success issuance, demonstrate separate bounded candidate
   execution: non-root, no network/credentials/host socket, dropped capabilities,
   fixed time/CPU/memory/process/output limits, read-only root/assets and an
   isolated writable output area. Trusted code hashes all source/font/reference
   bytes before and after execution and generates its own result records.
   Candidate JSON is not an attestation. Missing/duplicate/unselected/reclassified
   cases or any infrastructure failure must prevent success. Use synthetic
   renderer inputs for this T005 smoke; full comparison remains T015.
7. After that App has emitted the context, add **`Taiyou trusted validation / corpus-gate`**
   to Taiyou's required checks with **that exact App as expected source**, strict
   up-to-date checking and no bypass. Existing `Fast gate`, `Platform gate` and
   `Documentation structure` checks use GitHub Actions; retain them as additional
   source/build checks, not substitutes for the dedicated trusted context.
8. Save authenticated branch protection/ruleset responses from both repositories,
   approved revision/review URLs, check-run App ID, candidate/binary/policy/corpus
   hashes and run/job URLs. Verify `branches/main` is protected and
   `rules/branches/main` includes the effective rules. A maintainer must show a
   direct/bypass update and stale/self approval cannot satisfy the gate, and a
   same-name check from GitHub Actions cannot satisfy the App-bound requirement.

## Security evidence still required after setup

The candidate primitive suite covers changed asset bytes/hashes, changed thresholds,
supported-to-unsupported snapshot demotion, deleted selectors/cases, missing or
duplicate/unselected records, unapproved statuses, false snapshot hashes,
ART_REG_* variables and symlink escape. These are **not** trusted-deployment tests.

In the actual protected service, run deliberately tampered synthetic candidates
that edit their own policy and CI to suppress failures, forge snapshot hashes,
mutate mounted assets, and attempt credential/protected-storage access. Verify
read-only denials and unchanged pre/post hashes, trusted-computed result rejection,
bounded termination and denial of an identically named candidate-issued check.
Archive the rejection checks with their actual App ID. None of these isolation
or issuer-spoofing demonstrations has been run yet. Do not mutate upstream assets.

Until these steps and independent review are evidenced, inventory implementation
may be CI-verified, but trusted protection is not operational, T005 is not formally
accepted, and T015 remains blocked.

Primary GitHub documentation: [branch protection and expected check source](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
[ruleset availability](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets),
[organization-level rules](https://docs.github.com/en/organizations/managing-organization-settings/creating-rulesets-for-repositories-in-your-organization).
