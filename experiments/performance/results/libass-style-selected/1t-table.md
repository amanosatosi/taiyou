| Workload / state | Taiyou1T ms | libass RGBA ms | 1T/libass | Painted pixels T/L | libass images | Taiyou bottleneck (prior profile) |
|---|---:|---:|---:|---|---:|---|
| dialogue-1920 / same-time | 4.655 | 0.601 | 7.75x | 12849-12849 / 12774-12774 | 3-3 | composition |
| dialogue-1920 / sequence | 4.758 | 0.598 | 7.96x | 12849-12849 / 12774-12774 | 3-3 | composition |
| thick-borders-1920 / same-time | 7.104 | 1.527 | 4.65x | 31448-31448 / 31408-31408 | 9-9 | composition |
| thick-borders-1920 / sequence | 7.172 | 1.726 | 4.16x | 31448-31448 / 31408-31408 | 9-9 | composition |
| dialogue-3840 / same-time | 14.720 | 2.366 | 6.22x | 49029-49029 / 49081-49081 | 3-3 | composition |
| dialogue-3840 / sequence | 14.594 | 2.295 | 6.36x | 49029-49029 / 49081-49081 | 3-3 | composition |
| combined-abuse-3840 / same-time | 114.674 | 44.154 | 2.60x | 196920-196920 / 208166-208166 | 48-48 | composition |
| combined-abuse-3840 / sequence | 392.968 | 56.152 | 7.00x | 188341-196920 / 200101-208166 | 48-48 | span union + mask/geometry; composition |
| large-blur-3840 / same-time | 19.365 | 5.347 | 3.62x | 118005-118005 / 134210-134210 | 6-6 | composition |
| large-blur-3840 / sequence | 18.134 | 5.048 | 3.59x | 118005-118005 / 134210-134210 | 6-6 | composition |
| transforms-3840 / same-time | 20.004 | 5.611 | 3.57x | 55411-55411 / 57213-57213 | 6-6 | composition |
| transforms-3840 / sequence | 56.839 | 6.910 | 8.23x | 53931-56509 / 56411-58061 | 6-6 | span union + mask/geometry; composition |
| upstream-karaoke-1920 / same-time | 4.819 | 0.799 | 6.03x | 46426-46426 / 46423-46423 | 12-12 | composition |
| upstream-karaoke-1920 / sequence | 4.778 | 0.805 | 5.94x | 46426-46426 / 46423-46423 | 12-13 | composition |
