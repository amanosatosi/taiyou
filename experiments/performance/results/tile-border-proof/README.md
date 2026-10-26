# Discrete ellipse tile proof before production selection

Candidate d0173b4eb3533108aee65c938394491c9a06347e.
[Green focused run40362747460](https://github.com/amanosatosi/taiyou/actions/runs/40362747460),
job121712251908, artifact12668228959. Native renderer slice and workspace native
Clippy passed. Existing fill/curve/resource/capacity proofs passed. Two new border
tests passed in13.34s: all bit-range endpoints, plus72 fully admitted cases with
exact native mask bytes/bounds/strides/work and legacy peak/work witnesses.
352 restricted attempts returned identical error code/resource classification/
detail. The matrix includes holes, intersections, curves, negative positions,
four subpixel phases, asymmetric/thick/zero-axis/tiny radii and differing global
and remaining work limits. Other restricted successful attempts also compare
exactly; no fallback is permitted in the fully admitted cases.

This was test-only; production borders remained unchanged. Formatting is recovered
from the runner. Prior af5d71d run40362213114 stopped at native Clippy because a
test-only counter was exposed in a non-test build; no tests were represented as
passing in that run. The counter scope/read and optional publication accounting
were corrected in d0173b4. No reference assets, quota values, fixtures or protected
comparison rules changed. Performance and full historical/platform validation are
still pending; probe gates remain explicitly unsatisfied.
