# Taiyou

CPU-first ASS/SSA subtitle renderer: architecture and implementation blueprint.

**Current state (2026-10-17): documentation only.** There is no renderer, Cargo workspace, benchmark baseline, or verified compatibility claim yet. The supplied directory was empty and not a Git repository. All source paths in implementation tasks are **planned**, unless explicitly identified as files delivered by this blueprint.

Start with [AGENTS.md](AGENTS.md), then [the task index](docs/tasks/README.md). Assign **T001: repository foundation** first. Do not begin with blur, SIMD, or a complete ASS parser.

Reference source folders are now available locally in `C:\taiyou-time\refrences`. Use the [local reference map](docs/LOCAL_REFERENCES.md) before browsing online. Run `pwsh -NoProfile -File scripts/check-local-references.ps1` to verify the recorded entry-point files.

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

No project license has been granted by this blueprint. MIT OR Apache-2.0 is proposed for original Taiyou code; the repository owner must choose before distribution. Reference projects and fonts retain their own licenses. No reference implementation code or images are copied into this repository.
