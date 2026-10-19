# T017 corrective validation and warm seeking

Tested source `84bad36f26e7715824a935a03954ad27e76fdbe2`, base
`d5a545ac49aabbfc23e3895231d407da576e276f`. These are completed observations,
not approval of production automatic wrapping. Later evidence-only commits do
not relabel this measured source. See provenance.json for artifact hashes and
per-file receipts. Original measurement files are copied byte-for-byte; the
sanitizer excerpt is explicitly derived from the identified job log.

All native and ordinary tests passed on
[Linux](https://github.com/amanosatosi/taiyou/actions/runs/39304505409/job/118238496931),
[Windows](https://github.com/amanosatosi/taiyou/actions/runs/39304505409/job/118238496984)
and [macOS](https://github.com/amanosatosi/taiyou/actions/runs/39304505409/job/118238496957).
The [Platform gate](https://github.com/amanosatosi/taiyou/actions/runs/39304505409/job/118243181187),
[Fast/MSRV](https://github.com/amanosatosi/taiyou/actions/runs/39304505219/job/118237996539),
[documentation](https://github.com/amanosatosi/taiyou/actions/runs/39304505537/job/118237938823)
and [supplemental prerequisite checks](https://github.com/amanosatosi/taiyou/actions/runs/39304505471)
passed. Linux also built the pinned FreeType/HarfBuzz with ASan/UBSan, validated
exactly taiyou_font/lib and authenticated_outline/catalog/shaping test targets,
and passed all30 native font tests with leak detection and halt-on-error intact.

## P0 result

All three platforms pass all six staging tests. The native and Linux MSRV raw
reports show thin/thick default500-event failures at events441/235, with444/238
preparations and **one** attempt. ErrorCode/event/span/original detail remain
intact. Explicit4M caller-budget runs perform500 preparations, one attempt and
match minimal/adaptive pixels and diagnostics. Limits remain unchanged.

On each platform, minimal staging succeeds at29,445,497 logical transient bytes;
retained staging requires a minimal retry and succeeds with identical output.
Repeated public renders each use two attempts. One byte below that boundary
returns the minimal attempt's contextual LimitExceeded. This is a pinned-fixture
logical reservation boundary, not a process RSS or portable font budget promise.
Allocation provenance is tested without manufacturing an actual host OOM.

## Warm render/seek observations

Debug build; one process, pre-registered pinned Source Sans3.052 catalog and parsed
tracks retained. No frame-history cache. Eight requests per workload in the
order0.75,1.25,0.5,0,1.5,0.75,1,0.5 seconds. The first event's end was changed
only in these separately named experimental scripts to1s, exercising active-set
changes. Every warm request is checked against an independent fresh Renderer:
all32 RGBA byte comparisons and diagnostic comparisons pass. Every source and
frame SHA-256, CPU tick delta and raw timing is in linux/T017-warm-seek.jsonl.
The first warm sample includes initial rendering; these are not playback FPS.

| Workload | Warm median ms | Fresh median ms |
|---|---:|---:|
| One ordinary event | 4.10 | 3.98 |
| Eight ordinary events | 17.90 | 17.91 |
| 100-event thin-border typesetting | 278.26 | 278.31 |
| 100-event1920×1080 typesetting | 1666.32 | 1664.29 |

Medians cover the specified mixed timestamps, not a constant active-event count.
The high-resolution script uses32-unit Source Sans3, border2, shadow2 and visible
shadow, with98 explicitly positioned typesetting events and two automatic lines.
The process report records CPU/OS/font identity,100Hz CPU tick units and42,872KiB
peak RSS. CPU ticks exclude child sha256sum work; timing excludes hashing/fresh-frame
verification. RSS is process-wide, not attributable to individual workloads.
Logical reservation and per-stage call instrumentation were not enabled in this
warm experiment; use the separate instrumented cold-process report for those.

The [complete cold debug/release and raw-native-work report](../dc83cab/summary.md)
remains exact-source evidence. Its three completed reuse receipts here verify
production/build/input identity and hashes; current correctness tests still ran.
No known baseline timeout or failed-limit result was rerun or overwritten.

## Protected comparison

The [full comparator](https://github.com/amanosatosi/taiyou/actions/runs/39304505616/job/118238426803)
exited1 and uploaded its unmodified report/differences, artifact12213042517.
All104 inventory entries and187 frozen assets were verified. Visual101:
1exact,92unsupported,8infrastructure errors,0regressions,0crashes. Both parser
safety probes passed; one upstream source remains disabled. Case-by-case
statuses are unchanged from dc83cab, not enforced as permanent counts.

The errors remain embedded-font/efont-1000.png at asset verification (no explicit
fonts), faux-italics/cff-0000.png, font_nonunicode/shiftjis-1000.png and
ssa/alpha-{0000,1000,2000,3000,4000}.png (the latter seven FontUnavailable, exit30).
Full IDs have the regression/ prefix. No infrastructure result is a visual pass.
The [independent pinned libass oracle](https://github.com/amanosatosi/taiyou/actions/runs/39304505616/job/118238426953)
reproduced101/101 at both pass levels, separately from Taiyou and xy evidence.
[T016's supplemental five frozen cases](https://github.com/amanosatosi/taiyou/actions/runs/39304505809/job/118238250275)
also passed. No protected fixtures, expected pixels, classifications, tolerances,
comparison mapping or required coverage changed.

ADR-013/C4 is owner-approved, including recursive binary64 selection and the
complete Unicode15.1 Regex-Number profile. Production wrapping/storage/native
preflight/C7 admission remains incomplete; automatic0/1/3 stays disabled. The
Corpus gate correctly still requires the human's exact final-head ADR-011
acknowledgment after technical review. PR29 remains unmerged; T017 is not complete.
