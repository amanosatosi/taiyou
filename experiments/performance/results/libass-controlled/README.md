# Explicit style fields, missing event style selection: retained intermediate run

Commit d49c58425998d27e20f0de565d98c942386a12e6;
[Actions run 40315878362](https://github.com/amanosatosi/taiyou/actions/runs/40315878362),
job 121548225202, artifact 12647242256. No production renderer changes.
All 2604 samples completed; Taiyou 1/4-worker RGBA and diagnostic digests matched,
and all Taiyou RGBA digests matched the original sparse-input run.

This is NOT the final controlled workload comparison. Explicit style fields alone
did not fix libass's smaller painted area: the authored Event Format still omitted
Style. Local libass ass.c process_event_tail assigns Style only when that field
is present; ass_new_track installs its own built-in style at index zero. Thus the
named authored Default style was not selected by those events. The upstream
karaoke script already selected its style and is unaffected. Preserve these raw
samples and their caveat; do not use their ratios as controlled equal-work claims.

The next run explicitly selects Default on authored events, retains all full style
fields, and requires unchanged Taiyou RGBA against the first retained run. Fonts,
text/tags, event counts, dimensions, timestamps and sample counts stay unchanged.
