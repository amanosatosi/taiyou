| Workload / state | Taiyou1T ms | reference1T ms | libass+RGBA ms | 1T/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 3.750 | 5.573 | 0.794 | 4.72x |
| dialogue-1920 / sequence | 3.748 | 5.571 | 0.794 | 4.72x |
| dialogue-1920 / cold | 11.920 | 15.167 | 2.280 | 5.23x |
| thick-borders-1920 / same-time | 8.242 | 8.796 | 2.750 | 3.00x |
| thick-borders-1920 / sequence | 8.280 | 8.928 | 2.745 | 3.02x |
| thick-borders-1920 / cold | 48.163 | 48.282 | 4.549 | 10.59x |
| dialogue-3840 / same-time | 10.020 | 19.159 | 3.908 | 2.56x |
| dialogue-3840 / sequence | 10.035 | 19.420 | 3.887 | 2.58x |
| dialogue-3840 / cold | 37.634 | 50.328 | 5.627 | 6.69x |
| combined-abuse-3840 / same-time | 65.589 | 77.420 | 92.758 | 0.71x |
| combined-abuse-3840 / sequence | 379.975 | 398.151 | 99.366 | 3.82x |
| combined-abuse-3840 / cold | 357.359 | 374.246 | 95.955 | 3.72x |
| large-blur-3840 / same-time | 14.598 | 23.852 | 10.020 | 1.46x |
| large-blur-3840 / sequence | 14.397 | 25.311 | 10.018 | 1.44x |
| large-blur-3840 / cold | 107.904 | 119.397 | 12.279 | 8.79x |
| transforms-3840 / same-time | 15.491 | 24.713 | 11.185 | 1.39x |
| transforms-3840 / sequence | 50.296 | 65.583 | 13.389 | 3.76x |
| transforms-3840 / cold | 49.904 | 62.909 | 13.581 | 3.67x |
| upstream-karaoke-1920 / same-time | 3.428 | 5.484 | 1.283 | 2.67x |
| upstream-karaoke-1920 / sequence | 3.469 | 5.704 | 1.319 | 2.63x |
| upstream-karaoke-1920 / cold | 30.551 | 34.155 | 3.030 | 10.08x |
