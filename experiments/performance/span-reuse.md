# Reuse repeated native scan rows

The measured cached single-worker combined 4K changing-frame path still spends
385.90 ms in span union and 64.98 ms expanding borders. Thick-border 4K exceeds
the benchmark-only memory envelope. These observations justify reducing duplicate
intermediate spans before introducing parallel execution.

For identical horizontal intervals on consecutive scan rows, the discrete ellipse
width is non-increasing with distance from its centre. At any output row, the
nearest source row therefore contains every other expanded interval from that
vertical band. Emit that maximum interval once. The original final union and
coverage accumulation remain unchanged. This also applies to zero horizontal
radius; zero vertical radius retains the original producer.

Optional scratch uses fallible reservation and the existing transient limit.
Non-monotone kernels, no repeated rows, insufficient scratch, or an expansion
whose conservative envelope would grow all fall back to the original algorithm.
No production cap increases. Fewer generated candidates are actual eliminated
work, not relaxed accounting. Supplemental native-cell set tests compare with
the original expansion over varied radii, signs and interval patterns. Full
external comparison and recorded actual Taiyou frame digests remain independent
checks; neither becomes a new accepted oracle.

The focused benchmark records source rows, actual candidates and candidates the
original producer would have generated. Results are pending Actions execution.
