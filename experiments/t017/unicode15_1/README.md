# Unicode 15.1 prerequisite experiment

This directory is isolated from the public renderer. Automatic Core WrapStyle
0/1/3 and all Legacy wrapping remain disabled. ADR-013 and production C7 limits
still require owner confirmation. No runtime dependency is added.

`manifest.json` pins original source bytes, HTTPS acquisition locations, sizes
and SHA-256 values. The matching UAX versions are [UAX14 revision51](https://www.unicode.org/reports/tr14/tr14-51.html)
and [UAX29 revision43](https://www.unicode.org/reports/tr29/tr29-43.html).
The original property/test files are unmodified and marked `-text` to prevent
Git newline conversion. `data/LICENSE.txt` is the official Unicode License V3
permission notice retrieved with these data; original 2023 notices remain in
all upstream files. This data license is separate from Taiyou's implementation.

On GitHub Actions only, run:

```
python experiments/t017/unicode15_1/generate.py "$RUNNER_TEMP/t017-unicode"
```

The generator verifies every source before parsing it, sorts/merges adjacent
identical ranges, handles UnicodeData First/Last ranges, and emits a deterministic
compact JSON table and summary containing hashes and actual table sizes. Defaults
are explicitly recorded. The experiment uses integer scalar values and pinned
property lookups; it never reads Python/OS Unicode classification tables.

The independent default-rule probes consume **every unmodified upstream test
case**, comparing all code-point boundary offsets. They do not apply ASS space
consumption, mandatory escape handling, grapheme filtering, shaping clusters,
font fallback, dictionary tailoring or proposed DP decisions. Those require
separate Core fixtures. A passing probe establishes experimental algorithm/data
conformance only; it does not admit a production Rust line breaker or prove
its storage/CPU bounds. Any discrepancy fails the supplemental job and is recorded
by original file line number; expected boundaries are never rewritten.

Production admission still requires generated Rust storage/lookup review,
separate ASS-tailored/cluster-filtered integration fixtures, bounded native work
and a reviewed C7 policy, and final ADR-013 confirmation. The Python probe is
research tooling, not a new runtime implementation.
