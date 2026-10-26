# Initial sparse-style comparison: preserved observations

Candidate `e538ebccf4175108a0394467129a37a69648c499`;
[green run](https://github.com/amanosatosi/taiyou/actions/runs/40313590527),
[job](https://github.com/amanosatosi/taiyou/actions/runs/40313590527/job/121540529423),
[artifact](https://github.com/amanosatosi/taiyou/actions/runs/40313590527/artifacts/12646217256).
EPYC7763 VM with4 logical CPUs. libass0.17.5 API0x01705000, source commit
`f61db567e6593df3470e91594bcd4ad2d0473aff`, normal caches/ASM/default shaping.
Same native FreeType2.14.3/HarfBuzz14.5.0 prefixes, distro FriBidi1.0.13. The
FreeType pkg-config ABI version26.6.20 is not its project release number.

All2604 frames execute with31 samples/group. Zero process failures and zero
Taiyou1/4 RGBA or diagnostic digest differences. The measured owned-RGBA contract,
sample order, fonts and timestamps are shared. No libass visual mismatch is
reclassified as an invalid performance sample. libass API time and CPU RGBA client
time remain separate. Full raw samples, distributions and metadata are adjacent;
per-process RSS/stderr/CSV files remain in the linked artifact, without font/image
redistribution. Earlier syntax/provisioning failures contain no timing evidence.

Important experimental condition: authored cases copied the initial performance
harness's **sparse** style format. The normal engines apply different implicit
defaults to omitted alignment, margins, border style and colors. Local libass
`ass.c:process_style` initializes only scale for a new custom-format style; Taiyou
starts with its documented Core defaults. Do not present these ratios as an
equivalent complete-style effect comparison or change renderer behavior to align
them. The upstream karaoke fixture is already fully specified and unchanged.

Observed warm nonzero-alpha pixels (Taiyou/libass): dialogue1080p12849/3124,
dialogue4K49029/12184, borders1080p31448/9891, combined4K196920/163695,
karaoke46426/46423. These pixel-count differences are evidence, not rejected
samples. The table remains an honest measurement of these exact sparse inputs.

The final controlled run explicitly states **Taiyou's existing default style
values** in the authored fixtures. It retains text/tags/font sizes/event counts,
all fonts, timestamps and upstream inputs. Its Taiyou RGBA must match every
corresponding sample here exactly. Neither renderer is optimized or modified;
the final result clarifies experimental controls while preserving this run.
