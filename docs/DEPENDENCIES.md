# T002 dependency and native recipe candidate

State: proposed freeze, review and final three-platform verification pending. This document does not declare T002 accepted. [PR 10](https://github.com/amanosatosi/taiyou/pull/10) records current candidate SHAs, failures, run/job links and results. No local compilation or dependency installation is permitted.

## Exact selections

| Component | Version / source | Purpose and alternative |
|---|---|---|
| Rust | 1.99.1 in rust-toolchain.toml; declared MSRV 1.85.0, compatibility runner 1.85.1 | Current stable patch toolchain selected from [official releases](https://blog.rust-lang.org/releases/); existing Fast lane retains the bootstrap/MSRV check. T002 native lane checks the pinned compiler. |
| HarfBuzz | 14.5.0, commit `863d3f7787c6df18d20e4535c5906bf3eb803bd5` | OpenType shaping from immutable bytes; platform shaping and FreeType integration disabled. System/vendored arbitrary versions rejected. |
| FreeType | 2.14.3, commit `0a0221a1347e2f1e07c395263540026e9a0aa7c7` | Unhinted outline/gray raster foundation; not evidence of xy compatibility. Choose the reviewed FTL path before distribution. |
| harfbuzz-sys | =0.8.0, default features off | Servo raw C bindings; no bundled/coretext/directwrite/freetype feature. Safe adapter ownership remains T008. Handwritten bindings and the bundled native copy are alternatives deferred for review. |
| freetype-sys | =0.23.0, default features off | Stable raw C bindings; pkgconf must resolve the exact recipe prefix. Runtime version smoke catches silent bundled fallback. A maintained safe wrapper or generated minimal bindings remain review alternatives if this binding cannot satisfy the gate. |
| unicode-bidi | =0.3.18, std + hardcoded-data defaults | Rust bidi analysis; native FriBidi is the alternative not selected by ADR-002. |
| unicode-script | =0.5.8 | Rust script classification; ICU is the larger alternative not selected here. |
| Meson / Ninja | 1.12.1 / 1.13.2 | Upstream primary native build systems, avoiding HarfBuzz's community-maintained CMake fallback. |
| pkgconf / colorama | 3.0.7.post0 / 0.4.6 | Pinned cross-platform pkg-config executable and its Windows console dependency. |

Exact native archive SHA-256, URLs, font SHA-256 and Git blob identity are in [sources.lock.json](../build-support/sources.lock.json). They were acquired on [Actions run 39039556527](https://github.com/amanosatosi/taiyou/actions/runs/39039556527/job/117416074251), not invented or inferred from local archive labels. Native release tags were dereferenced through official GitHub APIs; the HarfBuzz tag was reported verified, while the FreeType/font tags were not reported signature-verified. Hash pinning is distinct from independent signature verification. The local reference trees are untouched and are not these build inputs.

Published crate checksums inspected from crates.io: harfbuzz-sys `b13f1cead31867a0d5b5e8905c262bc02b77701b8cd45ef402988f33745f9b7c`, freetype-sys `eab537ce43cab850c64b4cdc390ce7e4f47f877485ddc323208e268280c308ae`, unicode-bidi `5c1cb5db39152898a79168971543b1cb5020dff7fe43c8dc468b0885f5e29df5`, unicode-script `383ad40bb927465ec0ce7720e033cb4ca06912855fc35db31b5755d0de75b1ee`. Cargo-generated locks must match them. Pinned build-tool wheels use published PyPI SHA-256 values in [requirements.txt](../build-support/requirements.txt), binary wheels only, no undeclared dependency resolution.

## Build and linkage

All commands execute on Actions. [native-deps.yml](../.github/workflows/native-deps.yml) selects Windows 2022 MSVC x64, Ubuntu 24.04 x64 and macOS 15 ARM64. The macOS job checks the actual architecture. Compiler, image and Python versions are recorded; hosted images are not bit-identical compiler archives, and this is not a claim of binary reproducibility across OSes.

`python build-support/acquire.py` verifies the lock before extracting sources under RUNNER_TEMP. Tar extraction uses the data safety filter. The pinned font is Source Sans 3 Regular, release 3.052R, commit `ed1808970eb3c7301c9a523bee26473ba0bb62fa`, OFL-1.1. It is downloaded only on runners and is not added to Taiyou or substituted into libass-tests.

`python build-support/build-native.py` performs cold builds with Meson release/static, `--wrap-mode=nodownload`, `--libdir lib` and `auto_features=disabled`. Windows uses the MSVC environment and dynamic CRT (`b_vscrt=md`). FreeType disables Brotli, bzip2, PNG, zlib, mmap, HarfBuzz autohint integration and upstream tests. HarfBuzz disables platform/native integration, ICU/GLib/Graphite, alternative shapers, GPU/vector/raster/subset libraries, utilities/docs/introspection/tests. OpenType shaping stays enabled. The exact flag arrays and actual Meson options are reported by the driver. No runtime font/layout/render capability is promoted by these build choices; compressed font/container support and extra backends are not silently promised.

The install prefix is runner-local. PKG_CONFIG_LIBDIR/PATH are restricted to that prefix, all-static selection is requested, and the script verifies the selected prefix. Production workspace packages remain std-only until T008 adopts the reviewed dependencies; the separate dependency-smoke workspace tests the pinned bindings without creating an eighth production package. Rayon's and PNG's dependencies remain absent.

The temporary Rust FFI smoke checks the actual native versions and identical positive glyph counts for the pinned font through both libraries. Owned handles are checked and destroyed once; font bytes outlive FreeType faces. It also checks Latin/Hebrew bidi and Arabic/Han script classification. This is a narrow binding/build audit, not a reviewed production font adapter or shaping/renderer acceptance.

## Locks, maintenance and remaining gates

Lock generation and `cargo tree` run on Actions only; only the small resulting lock text is retrieved for reviewed inclusion. Final verification must run `cargo build --locked --workspace`, formatting, Clippy with warnings denied, workspace tests, exact seven-package graph validation and the native smoke on all three targets. The PR distinguishes initial discovery, failed candidates and final passed jobs. No cache hit substitutes for a clean-build result. Dependency/artifact caches remain runner-side; this initial audit uses cold native builds.

Maintenance metadata inspected 2026-10-17: HarfBuzz 14.5.0 is an active upstream release; the official [FreeType news](https://freetype.org/index) identifies 2.14.3 as a security-fix release. Servo's binding repository is active and unarchived, last pushed 2026-08-04; crates.io publishes harfbuzz-sys 0.8.0. freetype-sys 0.23.0 and unicode-bidi 0.3.18 have older releases; stable APIs alone do not prove ongoing maintenance. Native/API smoke and an advisory inventory are required, with maintainer review still open. Rust Unicode tables can lag HarfBuzz's Unicode 18 data; T008/T009 must report unsupported new scripts and freeze compatible analysis semantics rather than claiming matching Unicode coverage.

Remaining gates: actual three-platform and MSRV results, small lock inclusion, transitive license/advisory audit, assigned dependency/native/FFI review, and owner licensing choice before distribution. Neither owner-delegated routine integration nor candidate Actions jobs establish independent review or T005's protected validation.
