# Goals and release gates

There are no measured throughput or memory baselines yet. Numbers below are test scenarios or configurable policies, not guaranteed speed claims.

## G1. Vertical slice

P2 ends with a CLI rendering a fixed-font, UTF-8 ASS event with hard breaks, static solid color, alignment and explicit position into an owned RGBA frame and PNG. Transparent background, end-exclusive timing, clipping to viewport, negative coordinates, malformed input and no-active-event cases have tests. CoreV1 is the development profile. This slice proves stage boundaries, not general ASS compatibility.

## G2. Initial usable renderer

P5 is the minimum usable release gate: Windows/Linux/macOS headless builds; bundled/host-explicit fonts; ASS v4+ and common SSA v4 styles/events; shaping/fallback/bidi with pinned-font fixtures; wrapping/alignment/collision; colors/alpha/reset; borders/shadows; blur/be; position/move/fade/basic transforms; rectangular/vector clip and drawing; k/K/kf/ko karaoke; cache correctness; stable C ABI candidate; deterministic 1/2/4/8-worker output on each supported target. A manifest enumerates exact supported forms, parameter ranges and profiles. Unsupported legacy effects, codepages, vertical fonts or syntax cannot be hidden by a “supports ASS” label.

Before release every declared supported fixture passes its approved oracle, no previously passing fixture regresses, all corpus cases are accounted for, and there are no unexplained safety failures. LegacyXyV1 release claims require R001/R002/R004 evidence for each claimed family. If original VSFilter evidence remains unavailable, state that limitation. An explicitly scoped preview may ship CoreV1 earlier; it is not the initial compatibility release.

## G3. Performance

Measure ordinary dialogue, dense signs, many small events, complex animated KFX, large blur, static repeated frames, cold font loads, seeks and cache thrash. Collect wall time median/p95/p99, CPU time, peak RSS, accounted bytes, allocations, cache reuse, masks built, pixels touched and speedup/efficiency at 1/2/4/8 workers. On Ryzen 5 5500U-class hardware include 720p/1080p scenarios, memory/thermal configuration and sustained runs.

After R003 establishes variance, set per-workload budgets against the single-worker Taiyou baseline and separately pinned libass/xy reference runs with equivalent output/features. No “faster than libass” claim on a workload rendering fewer features. Small workloads must retain an inline path. Memory must obey configured Taiyou budgets; native RSS is separately measured. Performance gates must not trade away accepted output accuracy.

## G4. Compatibility

Prioritize pinterf xy-VSFilter historical behavior per explicit profile and pinned environment. Maintain separate libass and original VSFilter comparisons. Report exact matches, approved bounded differences, unsupported cases, regressions and missing evidence, with denominators. No universal pixel-perfect promise across all engines or Windows environments. No unsupported-feature decrease through manifest editing in ordinary PRs.

## G5. Advanced and long-term goals

After the usable release: gradients/per-vertex alpha, multi-border, furigana, curved placement, distortions and advanced compositing through existing stage seams. Study Mangetsu/libassmod syntax independently before a new extension profile. A future `.asa` parser may lower into shared scene types once an authoritative specification is available. FFmpeg/mpv integration follows ABI stabilization and host color-space policy, not API mimicry of libass.

## G6. Non-goals

No libass fork/translation, GPU-first engine, subtitle editor, scripting VM/Lua execution, network asset fetching, implicit tag-based file I/O, custom shaping algorithm, custom font format parser, or broad plugin system. No initial video decoding/YUV compositor, HDR/color-management engine, color emoji bitmap rendering, live font hot reload, persistent disk cache or arbitrary concurrent calls on one renderer. These can be separate reviewed projects if evidence warrants them.
