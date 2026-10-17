# Third-party intake notices (T002 candidate)

Original Taiyou licensing remains the repository owner's decision. This file is not a license grant or a completed distribution audit. Native sources, font bytes and dependency binaries are not vendored into this repository. CI tests do not authorize redistribution. See [dependency candidate](docs/DEPENDENCIES.md) and its outstanding review gates.

| Material | License choice / source notice | Obligations and status |
|---|---|---|
| HarfBuzz 14.5.0 | MIT-style project license and component notices; [COPYING at pin](https://github.com/harfbuzz/harfbuzz/blob/863d3f7787c6df18d20e4535c5906bf3eb803bd5/COPYING) | Retain copyright, permission and included component notices when distributing. Exact archive is hashed in build-support/sources.lock.json; file-level review remains open. |
| FreeType 2.14.3 | Proposed FTL choice, not GPL alternative; [LICENSE.TXT](https://github.com/freetype/freetype/blob/0a0221a1347e2f1e07c395263540026e9a0aa7c7/LICENSE.TXT), docs/FTL.TXT and additional notices | Retain license/copyright and required FreeType credit; check component notices. Optional compression/image dependencies disabled. Distribution choice/review pending. |
| harfbuzz-sys 0.8.0 | MIT OR Apache-2.0 | Published source license metadata inspected; retain selected license/notices. Bundled native/platform features off. |
| freetype-sys 0.23.0 | MIT | Binding license is distinct from native FreeType obligations. Bundled fallback is not an approved native recipe. |
| unicode-bidi 0.3.18; unicode-script 0.5.8 | MIT OR Apache-2.0, plus data provenance | Retain selected licenses and Unicode data notices. Declared metadata is not a full file-level audit. |
| Meson 1.12.1; Ninja 1.13.2 | Apache-2.0 | Runner-only build tools, versions and wheels hashed. |
| pkgconf 3.0.7.post0 | pkgconf's ISC-style notices and Python wrapper's license | Runner-only tool; wrapper/native notice review remains explicit. |
| colorama 0.4.6 | BSD-3-Clause | Runner-only Windows console dependency. |
| libz-sys 1.1.30 / its zlib 1.3.2 sources | MIT OR Apache-2.0 binding; zlib native license | Required transitively by freetype-sys, static vendored source selected. Retain native zlib notice separately from binding notices; source checksum is in the audit lock. |
| cc 1.8.0, find-msvc-tools 0.1.14, libc 0.2.191, pkg-config 0.3.34, shlex 2.0.1, vcpkg 0.2.15 | Declared MIT OR Apache-2.0 (vcpkg metadata uses MIT/Apache-2.0) | Full resolved metadata is archived in build-support/dependency-audit.json; license text and component review remain open before distribution. |
| Source Sans 3 Regular 3.052R | SIL Open Font License 1.1; [upstream license](https://github.com/adobe-fonts/source-sans/blob/ed1808970eb3c7301c9a523bee26473ba0bb62fa/LICENSE.md) | CI-only pinned font, no redistribution here. Retain OFL/copyright and obey reserved-name conditions for future distribution/modification. |
| Source Sans 3 Bold, same 3.052R pin; generated catalog OTC | OFL-1.1 | Runner-only T008 collection fixture; see docs/T008-CANDIDATE.md. No reference expected output is changed. |
| sha2 0.11.0 and its RustCrypto dependencies | MIT OR Apache-2.0 (see resolved per-package metadata) | SHA-256 font identity; exact checksums in root lock, notices/license choice require review. |
| icu_casemap 2.0.1 and ICU4X/data dependencies | Unicode-3.0 (see resolved per-package metadata) | Full default family case folding; retain code and Unicode data notices, no normalization substitution. |
| FontTools 4.60.2 | MIT | Runner-only OTC generator, exact wheel hash in build-support/requirements.txt. |

The native job prints every resolved Rust dependency's exact version, repository, declared SPDX license and rust-version from Cargo metadata; Cargo.lock records transitive checksums. Transitive/component/advisory review remains pending until that evidence is inspected. Do not present this intake list as blanket legal compatibility approval. No GPL VSFilter/libass source is copied, translated or linked into these recipes.
