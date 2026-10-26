# Frozen implementation, same-runner libass comparison

Frozen checkpoint: `2b80565e4998e22032d6bb4a446013886df7c62c`. Fast and the complete
[Windows/Linux/macOS platform lane](https://github.com/amanosatosi/taiyou/actions/runs/40308776541)
pass, including the authorized T017 hash intake and existing Linux MSRV/native
sanitizers. The fixed7440-frame cache/worker audit is green. The separate external
visual suite retains its known red mismatches; no acceptance policy changes.

The comparison adds callers only. CI checks the renderer/font/types/parser/C API,
CLI production sources, manifests/lock, build inputs and frozen validation files
against that checkpoint. No optimization or renderer redesign occurs here.
Taiyou uses its production timing-disabled path, default bounded128MiB cache,
workers1 and explicit4, default64-row batches. The heavy benchmark's explicit
512MiB/32M work allowance is recorded; production caps are unchanged.

libass uses the existing pinned commit from docs/references.lock.json, normal
production renderer/caches/CPU dispatch and complex shaping. No internal timers,
cache-limit changes or source modifications. Both link the same pinned FreeType
and HarfBuzz builds; the distro FriBidi version/notices and all build flags are
recorded. Only explicitly supplied identical font files are registered; host
font discovery is disabled for both. The actual API version and source release
are recorded instead of inferring an archive identity.

Seven cases cover1080p dialogue/borders,4K dialogue/stress/blur/changing geometry,
and the existing pinned upstream large-font karaoke/kf witness. It is not a
Mangetsu KFX port or a claim to contain the entire future feature load. Authored
workloads retain the existing performance harness's text, tags, font sizes and
event counts. The first sparse-style run is retained separately; final authored
styles explicitly state all frozen Taiyou defaults, and authored events explicitly
select that named Default style. The intermediate style-only run is also retained:
its missing event Style field still selected libass's built-in style zero. Every Taiyou RGBA digest must equal that prior
run. This is input specification, not a rendering-semantics correction. The upstream script
and font bytes are fetched/verified on CI from the existing immutable inventory,
never edited or uploaded as promoted references. All fonts, source hashes, sizes
and31 timestamps per group are shared by the callers and verified before/after.
Integer-ms timestamps avoid comparing Taiyou microsecond transforms with rounded
libass timestamps. Four playback scenarios and deterministic engine-order rotation
are recorded. No correctness test is reduced or replaced by this experiment.

The primary table compares a complete owned transparent premultiplied RGBA frame
every call. libass returns ASS_Image coverage, so a straightforward scalar C client
allocates/clears/composes those images each time, including unchanged frames.
Both modes include that output work; the client never reuses a finished frame.
libass native API time and client composition time are separately available.
This client is a measured adapter, not a claim to be every video host's compositor.
Different native mask/clip/metric semantics can produce different painted work;
nonzero-alpha counts are recorded and visual mismatches never invalidate a sample.

Warm scenarios have one full warmup frame. Cold means a fresh renderer with the
same admitted fonts/parsed track; cold-session values include renderer creation
(font selection/pool setup) plus first render. Font/source admission and process
elapsed/CPU/RSS are separate, not hidden inside a warm-render ratio. Checksums,
diagnostic SHA-256, scans and CSV writing are outside frame timing. Taiyou1/4
digests must match for every sampled frame; libass FNV is anti-elision only and
there is no cross-engine visual equality gate. Median/p95/p99/sample counts and
FPS equivalents are reported, with no noisy performance pass/fail threshold.

Existing frozen checkpoint stage measurements identify Taiyou's remaining span
union and composition costs. This comparison does not add internal libass stage
instrumentation or infer its bottleneck from an API total. No gap will be fixed
as part of the comparison; stop after reporting measurements and their limits.
