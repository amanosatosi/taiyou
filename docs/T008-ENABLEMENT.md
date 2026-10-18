# Provisional T008 CI enablement

Base `5676edf1573cacff7dcd217243f0a94a4cc6a87a`. The owner explicitly authorizes this separate, unmerged enablement proposal. It is not protected validation approval or task acceptance. Historical independent-review debt remains for merged PRs 8/9/10/12 (empty GitHub review lists).

The seven-package A3 internal DAG and all existing source/native tests remain required. The dependency checker admits only four exact optional dependencies in taiyou-font, with exact registry/version/default/feature choices; wrong owners, versions, sources, features, target/rename/kind and unknown packages fail negative tests. Fast fetches the committed lock on runners before its offline default-feature checks. The native matrix will exercise the native feature; default builds do not silently use a system FreeType.

Only taiyou-font changes its crate-level unsafe lint from forbid to deny so its isolated native module can explicitly allow justified FFI. Every other crate retains forbid; no unsafe Send/Sync is authorized. The implementation must keep public handles opaque and contexts neither Send nor Sync. This policy change and the native implementation require independent protected/native review before integration.

Review follow-up: `scripts/check-unsafe.py` enforces exact manifest lint inheritance and permits exactly the module-level exception in `crates/taiyou-font/src/native.rs`. It rejects other allow/expect/cfg_attr/macro exceptions, executable unsafe outside that file, code includes/custom module paths, workspace build/proc-macro scripts and custom target paths. Negative tests exercise each rejection. Strings/comments are lexically skipped, including nested comments and raw strings. This conservative source/manifest check runs in an always-selected Fast policy job and before native platform selection. Forbid/deny, compiler checks and warnings-denied Clippy remain required. This candidate-side scanner does not replace independently trusted validation, branch rules, or required GitHub reviews under T005.

The owner accepts the pinned sha2/ICU4X selections as proposed implementation dependencies in this follow-up; technical, dependency and license reviews remain required. This is not blanket dependency or protected-policy approval.

Exact intake:

| Dependency | License, purpose and alternatives |
|---|---|
| freetype-sys =0.23.0, optional, defaults off | T002 pinned MIT bindings; exact native FreeType remains 2.14.3 from the existing hashed recipe. No bundled/system substitution. |
| libz-sys =1.1.30, optional, static | T002 transitive static zlib requirement. Native zlib/binding notices remain separate. |
| sha2 =0.11.0, optional | MIT OR Apache-2.0, maintained RustCrypto SHA-256; MSRV 1.85. Published checksum `446ba717509524cb3f22f17ecc096f10f4822d76ab5c0b9822c5f9c284e825f4`. Avoids inventing cryptography; handwritten hashing and platform crypto were considered and rejected. New helper intake requires review; not represented as already selected by T002. |
| icu_casemap =2.0.1, optional, compiled_data, defaults off | Unicode-3.0, maintained ICU4X full default case folding; MSRV 1.82. Published checksum `2183be5a01b948227f1a521f4e586ee69828d2eecc1d2397ce686de68233d27d`. Avoids locale-dependent lowercase or an abandoned folding crate; a separately audited bounded Unicode table is the alternative. Folding adds no NFC/NFKC normalization. New helper intake and Unicode data obligations require review. |

Versions/licenses/checksums were read from the official crates.io version endpoints; resolution/lock generation and all execution occur on Actions only. No local dependency/toolchain installation is allowed. T008 will return only small generated locks/reports. Distribution licensing and the T005 independently trusted validation gate remain open.
