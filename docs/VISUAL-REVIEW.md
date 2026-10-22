# Visual compatibility review (T015 follow-up)

Scope: the maintainer's dedicated review PR, based on merged PR #32 at
`4c51ca61902e4c90930946892f47695a2d1f1317`. No renderer behavior changes.

The single `visual-review-SHA` Actions artifact has eight shallow behavior folders:
blur, karaoke, positioning, scaling, fonts, drawings, parsing and alpha. Each has
one short README.txt. Every successfully rendered corpus frame, including exact
matches, has adjacent `CASE-1-reference.png`, `CASE-2-taiyou.png`,
`CASE-3-diff.png`. Filenames preserve the upstream millisecond zero padding.
SUMMARY.txt reports current per-folder results, with case details in RESULTS.txt
and manifest.json. No supported/mismatch counts are hardcoded.

The optional `--visual-review PATH` comparator output retains images while the
existing renderer's temporary files exist. It does not rerender the corpus. The
legacy bounded raw artifact is retained unchanged. Strict unsupported cases gain
a diagnostic triplet only when the existing tolerant attempt succeeds and its
pixels validate; their filename includes `diagnostic-unsupported`. Neither
strict metrics nor classification receives tolerant results. Parser-only probes
and upstream-disabled sources remain separately reported without invented images.
Export failures are checked separately and cannot turn a red comparator green.

Reference PNGs are unchanged byte copies of the frozen libass-tests assets, at
the existing pinned revision. They are not authenticated xy-VSFilter captures.
Diffs reuse the existing absolute-difference display and arithmetic, including
alpha deltas in red. Oversampled frames use the existing normalized RGBA16 pixels
and display-only RGBA8 preview. No resizing, tolerance or reference replacement
is introduced. This is a maintainer-requested private CI review download; fonts
are not bundled and no public redistribution license is asserted.

Supplemental [Taiyou-owned reductions](../tests/taiyou/visual-review/README.md)
are candidate-only renders with no expected xy images, excluded from corpus
counts. WITNESSES.txt explains them collectively. Original fixture contents were
read at the pinned corpus revision because libass-tests is absent locally;
pinterf source study used the read-only local archive.

Development uses `visual-review-focused.yml` on the task branch: synthetic image
export tests, all existing comparison self-tests, five representative corpus
frames and the reduced witnesses. Once stable, the ordinary PR compatibility
workflow runs one complete comparison with all existing gates and raw failure
uploads preserved. A mismatch remains a gate-failing result. The full native
libass oracle remains separate. CI commands execute only on GitHub Actions.

The workflow/export retention proposal requires final human exact-head/base/
change-digest acknowledgment under ADR-011. The agent must not post it or merge
the PR. Existing inventory, frozen images, comparison algorithms, tolerances,
support classifications, tests, required checks and triggers remain intact.
