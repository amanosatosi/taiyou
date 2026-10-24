# T002 dependency and native recipe candidate

PR #35 host-font provider proposal adds OS services, not a different shaping
engine or Cargo dependency: Windows DirectWrite IDWriteFactory2/FontFallback
(Windows 8.1+ system API, Microsoft Windows licensing); Linux Fontconfig
libfontconfig.so.1 (2.11.91+ API, MIT license); macOS CoreText/CoreFoundation
(system frameworks, Apple platform licensing). Fontconfig is dynamically linked;
Linux packagers must supply its runtime library. These services select installed
font candidates only in CLI system mode. Linux links the versioned runtime SONAME
directly, without requiring a development symlink. Explicit
catalogs and corpus adapters do not call them. Alternatives are enumerating and
parsing every installed font, hardcoded fallback families, or replacing Taiyou's
shaper with libass; none is selected. The approach follows local libass 0.17.5
provider sources (ISC), without copying pinterf GPL code. The new native/API
boundary remains subject to PR review. Android has no adapter in this candidate;
its embedding host can continue supplying explicit catalogs.

Compatibility repair intake: `encoding_rs =0.8.35`, optional native-font feature,
default features disabled. Purpose: allocation-free Unicode-to-Big5/Shift-JIS
(and related Microsoft DBCS) code conversion before looking up non-Unicode font
cmaps. It does not decode subtitle input or change ASS Encoding policy. License:
`(Apache-2.0 OR MIT) AND BSD-3-Clause`; Unicode mapping data retains its upstream
notices. Its only dependency is the already present `cfg-if` (MIT OR Apache-2.0).
The pinned source is hsivonen/encoding_rs tag v0.8.35; Cargo resolves/checksums it
on Actions. Alternatives: OS iconv/Windows codepage APIs would add platform/locale
variation and FFI; handwritten per-font glyph tables are unacceptable. Existing
FreeType/HarfBuzz still own font parsing, glyph identities, metrics and shaping.

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
| libz-sys | =1.1.30 in the isolated audit; vendored zlib 1.3.2 | Unconditional freetype-sys dependency even with native FreeType compression disabled. Static feature and LIBZ_SYS_STATIC=1 prohibit a host zlib fallback. |
| Meson / Ninja | 1.12.1 / 1.13.2 | Upstream primary native build systems, avoiding HarfBuzz's community-maintained CMake fallback. |
| pkgconf / colorama | 3.0.7.post0 / 0.4.6 | Pinned cross-platform pkg-config executable and its Windows console dependency. |

Exact native archive SHA-256, URLs, font SHA-256 and Git blob identity are in [sources.lock.json](../build-support/sources.lock.json). They were acquired on [Actions run 39039556527](https://github.com/amanosatosi/taiyou/actions/runs/39039556527/job/117416074251), not invented or inferred from local archive labels. Native release tags were dereferenced through official GitHub APIs; the HarfBuzz tag was reported verified, while the FreeType/font tags were not reported signature-verified. Hash pinning is distinct from independent signature verification. The local reference trees are untouched and are not these build inputs.

Published crate checksums inspected from crates.io: harfbuzz-sys `b13f1cead31867a0d5b5e8905c262bc02b77701b8cd45ef402988f33745f9b7c`, freetype-sys `eab537ce43cab850c64b4cdc390ce7e4f47f877485ddc323208e268280c308ae`, unicode-bidi `5c1cb5db39152898a79168971543b1cb5020dff7fe43c8dc468b0885f5e29df5`, unicode-script `383ad40bb927465ec0ce7720e033cb4ca06912855fc35db31b5755d0de75b1ee`. Cargo-generated locks must match them. Pinned build-tool wheels use published PyPI SHA-256 values in [requirements.txt](../build-support/requirements.txt), binary wheels only, no undeclared dependency resolution.

## Build and linkage

Platform selection is conservative: every PR and main push enters the workflow. Font adapters, C ABI/FFI, all Rust/native source, manifests/locks, build recipes, workflow/selector policy and unknown paths select the existing full three-OS matrix. Only known prose-only changes skip compilation; dependency/license prose still selects it. The always-running Platform gate rejects selector failure or any failed/cancelled selected matrix job. Selector regression tests run on Actions before selection. These are candidate CI changes requiring protected workflow review, not independently trusted T005 enforcement.

All commands execute on Actions. [native-deps.yml](../.github/workflows/native-deps.yml) selects Windows 2022 MSVC x64, Ubuntu 24.04 x64 and macOS 15 ARM64. The macOS job checks the actual architecture. Compiler, image and Python versions are recorded; hosted images are not bit-identical compiler archives, and this is not a claim of binary reproducibility across OSes.

`python build-support/acquire.py` verifies the lock before extracting sources under RUNNER_TEMP. Tar extraction uses the data safety filter. The pinned font is Source Sans 3 Regular, release 3.052R, commit `ed1808970eb3c7301c9a523bee26473ba0bb62fa`, OFL-1.1. It is downloaded only on runners and is not added to Taiyou or substituted into libass-tests.

`python build-support/build-native.py` performs cold builds with Meson release/static, `--wrap-mode=nodownload`, `--libdir lib` and `auto_features=disabled`. Windows uses the MSVC environment and dynamic CRT (`b_vscrt=md`). FreeType disables Brotli, bzip2, PNG, zlib, mmap, HarfBuzz autohint integration and upstream tests. HarfBuzz disables platform/native integration, ICU/GLib/Graphite, alternative shapers, GPU/vector/raster/subset libraries, utilities/docs/introspection/tests. OpenType shaping stays enabled. The exact flag arrays and actual Meson options are reported by the driver. No runtime font/layout/render capability is promoted by these build choices; compressed font/container support and extra backends are not silently promised.

The install prefix is runner-local. PKG_CONFIG_LIBDIR/PATH are restricted to that prefix, all-static selection is requested, and the script verifies the selected prefix. Production workspace packages remain std-only until T008 adopts the reviewed dependencies; the separate dependency-smoke workspace tests the pinned bindings without creating an eighth production package. Rayon's and PNG's dependencies remain absent.

The temporary Rust FFI smoke checks the actual native versions and identical positive glyph counts for the pinned font through both libraries. Owned handles are checked and destroyed once; font bytes outlive FreeType faces. freetype-sys omits FT_Library_Version, so the audit declares that single stable C diagnostic function using the binding's FT_Library/FT_Int types. This additional unsafe declaration requires the same native review. It also checks Latin/Hebrew bidi and Arabic/Han script classification. This is a narrow binding/build audit, not a reviewed production font adapter or shaping/renderer acceptance.

libz-sys vendors zlib 1.3.2 inside its crates.io archive, SHA-256 `f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19`; the lock checksums that complete source archive. The smoke asserts zlibVersion equals 1.3.2, and the driver reports the actual vendored header version. This transitive dependency does not enable FreeType's compressed-font support. macOS uses deployment target 15.0 consistently for native and Rust code; older macOS deployment is untested. Verified source trees are moved to short runner-local paths to keep HarfBuzz's generated command below Windows command-length limits; contents remain unchanged.

## Locks, maintenance and remaining gates

Lock generation and `cargo tree` run on Actions only; only the small resulting lock text is retrieved for reviewed inclusion. Final verification must run `cargo build --locked --workspace`, formatting, Clippy with warnings denied, workspace tests, exact seven-package graph validation and the native smoke on all three targets. The PR distinguishes initial discovery, failed candidates and final passed jobs. No cache hit substitutes for a clean-build result. Dependency/artifact caches remain runner-side; this initial audit uses cold native builds.

Maintenance metadata inspected 2026-10-17: HarfBuzz 14.5.0 is an active upstream release; the official [FreeType news](https://freetype.org/index) identifies 2.14.3 as a security-fix release. Servo's binding repository is active and unarchived, last pushed 2026-08-04; crates.io publishes harfbuzz-sys 0.8.0. freetype-sys 0.23.0 and unicode-bidi 0.3.18 have older releases; stable APIs alone do not prove ongoing maintenance. Native/API smoke and an advisory inventory are required, with maintainer review still open. Rust Unicode tables can lag HarfBuzz's Unicode 18 data; T008/T009 must report unsupported new scripts and freeze compatible analysis semantics rather than claiming matching Unicode coverage.

Runner-generated Cargo.lock files are included for the root workspace and isolated audit. [dependency-audit.json](../build-support/dependency-audit.json) archives the 2026-10-17 OSV Rust/PyPI query and all resolved Rust license/MSRV declarations from candidate `0f37aa173cd3dce609c61d88368bb18a79455ee3`: 15 package/version queries returned no matching advisory IDs. This is a timestamped service result, not proof of absence of vulnerabilities. Native release/security notices and component licenses require separate review. Cargo tree contains cc 1.8.0, find-msvc-tools 0.1.14, libc 0.2.191, libz-sys 1.1.30, pkg-config 0.3.34, shlex 2.0.1 and vcpkg 0.2.15 in addition to the selected bindings/Unicode crates. Exact checksums and declared licenses are preserved in the lock/audit files. No unlisted optional Rayon/PNG dependency is introduced.

Initial cold smoke and locked workspace checks passed on Linux and macOS ARM64 at `0f37aa173cd3dce609c61d88368bb18a79455ee3`: [Linux + exact Rust 1.85.0 MSRV + advisory audit](https://github.com/amanosatosi/taiyou/actions/runs/39041025417/job/117420399738), [macOS ARM64](https://github.com/amanosatosi/taiyou/actions/runs/39041025417/job/117420399987). Both report FreeType 2.14.3, HarfBuzz 14.5.0, zlib 1.3.2 and 2478 glyphs. Windows failed at HarfBuzz command length on that candidate. The PR records subsequent repaired candidate evidence; no whole matrix pass is claimed here.

Remaining gates: final three-platform result on committed locks, assigned dependency/native/FFI and component-license review, and owner licensing choice before distribution. Neither owner-delegated routine integration nor candidate Actions jobs establish independent review or T005's protected validation.

## T017 Unicode prerequisite data (experimental)

Unicode15.1 UAX14 revision51/UAX29 revision43 data and unmodified conformance
inputs are pinned by individual SHA-256/byte counts in
[the experimental manifest](../experiments/t017/unicode15_1/manifest.json).
The original files retain their notices and the [Unicode License V3](../experiments/t017/unicode15_1/data/LICENSE.txt)
permission notice is distributed alongside them. Acquisition locations, generation
procedure and isolated conformance scope are in the [prerequisite README](../experiments/t017/unicode15_1/README.md).
This adds data and Actions-only Python probes, with no Rust runtime dependency.
Host Unicode databases are an excluded alternative because their revision varies.
Using a new external line-breaking crate remains an alternative requiring its
own version/license/data/conformance and storage review. No production wrapping
admission or Core/Legacy policy change follows from importing these files.
