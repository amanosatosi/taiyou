# T017 independent-review corrections

This is a corrective checkpoint on PR29, based on integration commit
`d5a545ac49aabbfc23e3895231d407da576e276f` and reviewed candidate
`f41261ddebc61b1ebb77e6175aba619f0d45a41e`. It does not complete T017 or admit
production automatic wrapping. ADR-012/F1 remains accepted; ADR-013/C4 and new
C7 wrapping defaults remain proposed. All protected comparison inputs are unchanged.

## P0: typed, retention-dependent recovery

The previous public render wrapper retried any LimitExceeded/AllocationFailed
after optional staging. This replayed the 500-event thin/thick default segment
failures: previously 888/476 event preparations instead of 444/238. Segment
capacity is fixed and releasing layouts cannot increase it.

The correction carries narrow internal ResourceFailure provenance through the
existing opaque error wrappers. Checked transient-byte failures and fallible
allocation failures can qualify; fixed count/geometry/segment limits do not.
Mixed count/byte checks test the fixed condition first. Font-to-render and
source-map conversions preserve provenance. The font-context cap established
before optional retention is also independent of it. There is no message-string
classification, new C7 code, comparator mapping, limit increase or normative
contract change.

Only a memory-classified failure while optional slots/layout ownership exists
causes one minimal-staging retry. The old attempt is returned/dropped completely
before retry: contexts, diagnostics, local Arcs, paths, masks and operations do
not cross attempts. The optional-ownership flag is cleared when slots are dropped
before composition. Output reservation/publication remains transactional.

Additive stage tests count whole preparation attempts and per-event preparations,
compare the complete first-attempt error (including native evidence/event/span),
preserve default segment failures and compare expanded4M minimal/adaptive output.
A binary search finds the pinned small fixture's actual logical-memory boundary:
minimal succeeds there, retained staging fails a checked reservation, and the
public retry must produce identical pixels/diagnostics. One byte below the
boundary returns the minimal attempt's original error. Repeated and fresh
renders, full/partial/minimal retention and stable seeking remain independently
covered. Allocation classification is tested without exhausting host RAM; actual
host-OOM injection is not claimed.

## Wrapping prerequisites, not production wrapping

The pinned Unicode source/test data, license and generator live in
[unicode15_1](../../experiments/t017/unicode15_1/README.md). All unmodified upstream
cases are exercised independently of ASS tailoring and shaping restrictions.
The line corpus's own header selects UAX14 Example7 numeric tailoring; the probe
makes this explicit and keeps default pair rules separate. This needs explicit
identification in final Core policy confirmation, not silent admission.

Generated JSON uses 168,471 bytes for 8,969 merged ranges: LB2848, GCB1371,
EAW1169, EP78, InCB201 and general-category3302. SHA-256:
`08871fae7d3c776d7aee03cf70bf3dca0a512fdd6fcf4f7721dbd22c9e427d16`.
These are experimental JSON tables, not admitted production Rust storage. The
Python rule probe is not the public line breaker, and its direct contextual
scans are not a claim of production linear-time behavior.

Native-work experiments now charge every HarfBuzz attempt at the shared native
entry, including failed fallback candidates, font-group context reshapes and
empty-line metric work. Calls/context/range bytes are checked before native
entry. Every actual raw output glyph is counted before publication, including
rejected output. Counts use checked arithmetic. The raw count is unknowable
until HarfBuzz returns: this experiment does not claim a pre-shape hard bound
on font-driven substitution expansion. Once exhausted, the context rejects
further native work. The public renderer never enables experimental budgets.

The advances/full-boundary equivalence matrix remains intact. New mixed-script
negative tests compare both paths' raw work, exercise zero call/context/range/
raw-output budgets, and prove that rejected fallback work exceeds the final
published glyph count. A separate bounded triangular table shapes each candidate
once, retains only a logical width, and times a backwards minimum-line search
independently. This feasibility search does not assume monotonic widths and
is not an implementation/admission of the proposed mode0/3 DP comparator.

## Benchmark provenance and acceptance

The owner authorized reuse of hash-verified original/reviewed samples. Historical
manifest authority is pinned to reviewed commit f41261d; every saved payload,
source instrumentation identity, dependency/font policy and actual generated
workload hash is checked before reuse. Original CPU/OS/profile/raw timing/RSS/
reservation/error/hash evidence stays intact and separately labeled. Changed
candidate binaries and all candidate samples are always newly executed. Missing,
corrupt, mismatched and incomplete evidence fail closed. Historical and current
wall times are different hosted-runner observations, not simultaneous paired
performance claims. No original timeout or failed500-event record is overwritten.

Actual final validation, raw measurements and remaining decisions will be linked
here after the candidate's Actions jobs complete. Production automatic modes
0/1/3 remain UnsupportedFeature. Legacy L01-L03 require authenticated xy captures
and separate admission. No acknowledgment or merge is performed by an agent.
