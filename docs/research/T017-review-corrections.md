# T017 independent-review corrections

This is a corrective checkpoint on PR29, based on integration commit
`d5a545ac49aabbfc23e3895231d407da576e276f` and reviewed candidate
`f41261ddebc61b1ebb77e6175aba619f0d45a41e`. It does not complete T017 or admit
production automatic wrapping. ADR-012/F1 remains accepted; the owner has now
confirmed ADR-013 and authorized its C4 amendment. New C7 wrapping defaults and
production activation remain unapproved. All protected comparison inputs are unchanged.

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
makes this explicit and keeps default pair rules separate. The owner explicitly
approved this complete Regex-Number/LB13 profile in ADR-013; that contract
approval does not admit the experimental tables or enable production wrapping.

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

## Completed measurements and remaining acceptance gates

The [raw measurement report](../../experiments/t017/results/dc83cab/summary.md)
contains every cold-process debug/release sample, environment, input/frame hash,
CPU/RSS observation, logical reservation and native-work record. Its measured
production source is dc83cab; later test/documentation changes do not relabel
these measurements as newly executed. The source job completed the experiments
but subsequently failed the new synthetic fixture's provenance check. The
fixture was corrected, preserving ingestion validation. This distinction is
recorded in the report and reuse metadata; that job is not sanitizer acceptance.

Default500-event thin/thick failures now prepare444/238 events instead of
888/476, with the same failing event441/235 and unchanged1M segment default.
The explicit4M rows succeed, preserving exact current/reviewed RGBA and all
staging variants. Full/partial/minimal256-event staging uses256/439/512 shaping
calls and23,235,683/17,447,759/16,417,346 logical transient bytes. The original
180-second5,000-event timeout remains incomplete evidence. A thin100-event cold
wall-time outlier remains visible; CPU and renderer-stage evidence do not
establish its cause. No universal speedup or playback-FPS claim is made.

The isolated Unicode probe passes all1,187 original grapheme and10,274 original
line cases under the latter corpus's declared numeric profile. A separate `%(`
test distinguishes default pair rules. This is experimental Python conformance,
not production Rust conformance or ASS/cluster-filtered integration admission.
Original arithmetic/nonmonotonic/source/fallback equivalence tests remain intact.

The raw-work probe exposes a spaced-CJK fallback workload with3,923,700 native
attempts,1,758,299,100 cumulative context bytes and19,110,900 raw glyphs. Thus
candidate/returned-glyph counts alone cannot justify C7. The report proposes
explicit values for further caller-budget trials, **not production defaults**.
Direct Arabic/Indic-only raw-budget trials, realistic CJK family/fallback
configurations, preflight for candidate/storage work and excessive native glyph
expansion, production table lookup/storage review and separate ASS-tailored/
cluster-filtered integration remain incomplete. Native attempts/context/range
bytes are preflighted in the experiment; raw glyphs are charged after return.

The [completed corrective validation and warm/seek report](../../experiments/t017/results/84bad36/summary.md)
records passing Linux/Windows/macOS, MSRV and four-target ASan/UBSan checks,
P0 attempt/error evidence and32 exact fresh-versus-seeking frame comparisons.
Final-head gates and full external comparison/oracle evidence are also reported
on [PR29](https://github.com/amanosatosi/taiyou/pull/29) with exact job links and
candidate SHA. Successful current-policy frame hashes
must agree across staging; the approved pre-F1 footprint difference is kept
separate. Existing required checks and the four-target font sanitizer remain
intact. Full comparator nonzero status and artifacts remain visible. Its eight
existing font/inventory errors are reported separately from unsupported frames.

Production automatic modes0/1/3 remain UnsupportedFeature. Legacy L01-L03 require
authenticated xy captures and separate admission. Owner-approved ADR-013/C4 does
not approve C7 or production activation. T017 remains a corrective checkpoint;
the final exact-head ADR-011 acknowledgment and technical review are human gates.
No acknowledgment or merge is performed by an agent.
