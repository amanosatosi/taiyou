# Taiyou

CPU-first ASS/SSA subtitle renderer: architecture and implementation blueprint.

**Current state (2026-10-17): T001 foundation proposed; acceptance pending.** The seven-package Rust workspace and CI checks are present. The CLI supports `--version` and explicitly fails rendering requests. There is no renderer, parser, benchmark baseline, or verified compatibility claim. Other task source paths remain **planned** until their implementation is accepted.

Start with [AGENTS.md](AGENTS.md), then [the task index](docs/tasks/README.md). T003 waits for accepted T001; its exported shared types require architect review before T004 starts.

Reference source folders are now available locally in `C:\taiyou-time\refrences`. Use the [local reference map](docs/LOCAL_REFERENCES.md) before browsing online. Run `pwsh -NoProfile -File scripts/check-local-references.ps1` to verify the recorded entry-point files.

**Development is CI-first.** Git is now initialized and the project has been uploaded to GitHub. The user's PC is for editing and lightweight checks only because storage is limited. Compilation, native dependencies and renderer/test execution belong in GitHub Actions; do not set up local builds. See the [execution policy](docs/DEVELOPMENT.md#d0-execution-location-and-local-storage). Workflow implementation and successful CI runs are still task deliverables, not implied by this policy.

| Document | Purpose |
|---|---|
| [Architecture](docs/ARCHITECTURE.md) | Pipeline, modules, ownership and execution |
| [Contracts](docs/CONTRACTS.md) | Normative types, arithmetic, lifetime and stage boundaries |
| [Caching](docs/CACHING.md) | Keys, bounds, invalidation and concurrent publication |
| [Goals](docs/GOALS.md) | Release scope and measurable gates |
| [Roadmap](docs/ROADMAP.md) | Ordered milestones and task dependencies |
| [Guidelines](docs/GUIDELINES.md) | Engineering rules |
| [Testing](docs/TESTING.md) | Immutable references and protected validation |
| [Compatibility](docs/COMPATIBILITY.md) | Behavioral authority and known disagreements |
| [Sources](docs/SOURCES.md) | Inspected evidence, revision pins and license review |
| [Development](docs/DEVELOPMENT.md) | Agent roles, branches and GitHub Actions plan |
| [Decisions](docs/decisions/README.md) | Accepted choices and bounded experiments |

Run the blueprint checks with `pwsh -NoProfile -File scripts/check-blueprint.ps1`. This checks documentation structure, links, task dependencies and reference pins; it does not validate a renderer.

The [Blueprint workflow](.github/workflows/blueprint.yml) runs this cheap check without Rust setup. The [Fast workflow](.github/workflows/fast.yml) selects source changes conservatively, checks the [A3 dependency graph](docs/ARCHITECTURE.md#a3-modules-and-allowed-dependencies), and runs formatting, clippy, workspace tests and CLI smoke on an Actions runner. Run **all Cargo commands on CI only**, including `cargo run -p xtask -- check-workspace`. Rust 1.85.1 is a foundation bootstrap version; T002 owns the lasting toolchain and lockfile policy. No external Rust dependencies or build cache are needed for T001. Workflows and selectors require maintainer review; T005 must establish actual protected validation and repository rules.

No project license has been granted by this blueprint. MIT OR Apache-2.0 is proposed for original Taiyou code; the repository owner must choose before distribution. Reference projects and fonts retain their own licenses. No reference implementation code or images are copied into this repository.
