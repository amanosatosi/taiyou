# Proposed temporary draft performance probe deferral

The maintainer separately authorized this exact proposal in the current chat on
2026-10-26, limited to the scope below. It implements the owner's requested
focused exploration followed by one full candidate checkpoint. AGENTS.md's
protected CI policy applies outside this expressly authorized draft exception.

Scope: only draft PR36, head branch perf/cache-multithread, and a head commit
explicitly marked [perf-1t-probe]. The conservative platform selector, unsafe
checks, full-comparison self-tests, baseline authority, all triggers, Fast gate,
Corpus policy gate and all tests/fixtures/algorithms/thresholds stay intact.

On an explicitly marked probe, defer all three heavy platform jobs together and
fail Platform gate with an explicit deferral error. Fail the full external
comparison before compilation with an explicit deferral error. Neither gate can
report a pass or a visual result from a focused run. Existing failure artifacts
from completed full runs stay unchanged. The supplemental paired performance
workflow runs focused native correctness and Taiyou1T/libass measurements.

An unmarked commit or manual full dispatch executes the original complete jobs.
A candidate is not a validated checkpoint until its unmarked full run completes.
Remove these temporary deferral clauses before leaving draft/final review.

The patch changes only the two workflow job admission points and failing gate
explanations, not comparison code or support/failure policy. It adds no dependency.
