# Integrated closed-fill slice: exact proof, provisional performance result

Candidatecdfe868392f465d71201ffac7b2bf6fd33d3fdfe; paired referencee1316cdd8f2eaea4d83b7a1ac3d08a4d0efd1762.
[Green run40356262877](https://github.com/amanosatosi/taiyou/actions/runs/40356262877),
job121689440482, artifact12667775369. All4340 primary samples and the additional
changing profiles match candidate/reference RGBA and diagnostic SHA-256; original
Taiyou benchmark pixels and normal cache OFF/ON resource results are unchanged.
Native curve, witness/rejection, cache/seek/font/document/retained-frame/worker tests
pass. CI formatting is recovered/applied. Full historical/platform gates remain
deferred and unsatisfied; this is not a kept final checkpoint.

The production candidate selects recursive tiles only for closed native body fill.
It shares the existing curve polynomial, relative-origin truncation and endpoint
emission with the retained producer. Borders, open contours and optional scratch
declines retain the legacy path. Exact old quota witnesses remain in force even
when actual storage changes. The retained body remains a direct unit-test oracle.

Paired median ms:

|Workload/state|Before1T|Fill-tiled1T|Stock direct|Mangetsu direct|Mangetsu RGBA|
|---|---:|---:|---:|---:|---:|
|Dialogue4K cold|38.701|37.903|0.817|1.557|1.671|
|Stress4K changing|394.543|388.484|2.100|3.098|24.135|
|Stress4K warm|63.195|63.052|0.288|1.266|20.942|
|Transforms4K changing|51.642|51.232|0.896|1.172|1.779|
|Borders1080p warm|8.704|8.738|0.122|0.497|0.654|

The whole-frame gain is small and ordinary workloads are essentially unchanged.
This validates the smallest exact tile slice; it does NOT justify retaining a
large architecture change on performance alone. Keep/final-checkpoint decision
waits for removing the dominant border-expanded span path; remove/reconsider the
architecture if that does not produce material benefit.

One post-sample stress observation has2 tiled fills,0 fallbacks,216 empty regions,
62 solid regions,69 single-segment and302 complex leaves; tile scratch peak894560B.
The legacy resource witness is still248414936B, intentionally conservative. Actual
expanded candidates remain4.24M because border production is unchanged; union
still236.63ms (row bucket118.81ms, sorting111.58ms). The next experiment targets
that whole class of work, not allocator/SIMD tuning. All samples, areas/images,
distributions, native versions/pins and further changing-stage observations remain
adjacent or in the artifact. Host composition remains secondary.
