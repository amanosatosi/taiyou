# Engineering guidelines

## Implementation discipline

Follow [AGENTS.md](../AGENTS.md) and one [task](tasks/README.md). Contracts are a shared dependency, not an agent's private implementation choice. Public interfaces document units, ownership, errors and supported profile. Prefer small concrete functions and enums over trait hierarchies. Abstract only the native font/raster boundary and proven interchangeable policies. No async runtime, ECS, plugin registry or expression-language interpreter.

Use stable Rust, edition 2024 after T002 verifies the minimum compiler and dependency matrix. T002 pins an exact `rust-toolchain.toml` and lockfile; do not say “latest” in reproducible builds. Formatting and clippy are gates. Avoid public fields for types with invariants, unchecked casts, hidden unit conversions, interior mutability on shared outputs and `unwrap`/`expect` on untrusted data. Tests may unwrap when failure is the assertion. Constructors perform finite/range validation once; internal helpers may rely on documented validated inputs.

Error enums have stable machine codes and human details. Parsing recovery reports source spans and continues only where the next record boundary is known. Separate recoverable unsupported tags from fatal invalid buffers/limits. Keep deterministic diagnostics. Do not log from inner pixel/glyph loops; aggregate counters and expose optional structured stage timings.

## Native and unsafe boundaries

Only `taiyou-font` native adapter, `taiyou-capi`, and later measured SIMD modules may contain unsafe. Every block states pointer validity, extent, alignment, aliasing and lifetime proof. Wrappers own cleanup with RAII and check all native return values. FreeType library/face objects are worker-local; immutable bytes can be shared. No unsafe Send/Sync declarations without specific architectural review and tests. Rust FFI catches unwind but cannot recover arbitrary memory faults. Never expose a C pointer into temporary/scratch memory.

Pin and disclose native dependency versions and build flags. Prefer the same FreeType/HarfBuzz configuration in reference CI on all platforms. Runtime native version mismatches fail reproducible test mode with a diagnostic. Production dynamic linking may be supported with explicit version reporting; it cannot claim identical reference output without testing.

## Allocation and numeric work

Use checked area, stride, offset and support arithmetic. Fallible reserve before large vectors. Budget before native calls and mask allocations; bounded buffers do not prove total RSS bounds. Reuse scratch after high-water checks, shrink oversized scratch at barriers, never return references to it. Keep geometry color-free. Avoid copying a whole frame per effect or per tile; effects consume local bounded masks.

Use scalar integer blend/coverage kernels as the oracle. SIMD must have scalar fallback, runtime feature dispatch, unaligned/tail cases and byte equality tests. Never compile baseline binaries with target-cpu=native for distribution. Unsafe architecture intrinsics stay behind target checks; NEON and x86 paths share exact rounding. Floating transform kernels cannot change rounding, FMA behavior or profile tolerances as an optimization.

## Testing and measurement

Each task supplies tests of observable behavior, edge cases and failures; do not write assertions merely restating private implementation. Compare full buffer including alpha and initialized padding where relevant. Test repeated timestamps, backward seeks, cache off, tiny budgets and multiple worker counts as stages become concurrent. Do not make an ignored test stand in for unsupported-case reporting. All external expected assets stay immutable.

Optimize only after collecting stage time, pixels touched, cache misses, construction count, allocation volume and peak retained/transient bytes. Benchmark cold and warm cases separately on a controlled machine. Report distributions and raw data; neither a single fast run nor noisy hosted-runner timing proves an improvement. Prefer reduced work to more threads. Keep benchmarks outside correctness gates unless the controlled-runner policy explicitly enables a calibrated threshold.

## Portability and dependencies

Use fixed-width units for persisted data and C ABI; usize only for in-memory extents with checked conversion. Do not assume Windows path encoding, little-endian words for RGBA, OS font ordering, locale-specific numeric parsing, ASCII text, x86 SIMD or 64-bit pointer size. No implicit font discovery in deterministic tests. File paths are host/CLI inputs, not interpreted from subtitle strings.

Every new dependency proposal records purpose, pinned version, transitive license review, maintenance status, native requirements, security advisories and impact on Windows/Linux/macOS. Prefer existing dependencies over new helpers. T002 freezes the first set; later changes are separate reviewed tasks. Original code is independently implemented; do not translate GPL VSFilter code into Rust. See [source obligations](SOURCES.md#s4-license-and-reuse-policy).

## Required report

Give task ID, integration base, commit IDs if Git exists, files changed, exact commands and outcomes, reference/profile versions, failures and unsupported cases, limits and follow-ups. For performance changes include controlled hardware/build configuration and before/after raw result paths. State what was not run and why. No generated reference replacement is part of an ordinary implementation report.
