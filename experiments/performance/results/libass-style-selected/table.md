| Workload / state | Taiyou1T ms | Taiyou4T ms | libass+RGBA ms | MT/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 4.655 | 4.863 | 0.601 | 8.10x |
| dialogue-1920 / sequence | 4.758 | 4.862 | 0.598 | 8.13x |
| dialogue-1920 / cold | 14.267 | 13.010 | 1.513 | 8.60x |
| thick-borders-1920 / same-time | 7.104 | 7.407 | 1.527 | 4.85x |
| thick-borders-1920 / sequence | 7.172 | 8.404 | 1.726 | 4.87x |
| thick-borders-1920 / cold | 43.844 | 44.403 | 2.585 | 17.18x |
| dialogue-3840 / same-time | 14.720 | 14.358 | 2.366 | 6.07x |
| dialogue-3840 / sequence | 14.594 | 14.395 | 2.295 | 6.27x |
| dialogue-3840 / cold | 42.670 | 43.425 | 3.354 | 12.95x |
| combined-abuse-3840 / same-time | 114.674 | 77.315 | 44.154 | 1.75x |
| combined-abuse-3840 / sequence | 392.968 | 361.472 | 56.152 | 6.44x |
| combined-abuse-3840 / cold | 386.154 | 362.152 | 46.993 | 7.71x |
| large-blur-3840 / same-time | 19.365 | 18.592 | 5.347 | 3.48x |
| large-blur-3840 / sequence | 18.134 | 18.255 | 5.048 | 3.62x |
| large-blur-3840 / cold | 103.452 | 113.471 | 6.382 | 17.78x |
| transforms-3840 / same-time | 20.004 | 20.831 | 5.611 | 3.71x |
| transforms-3840 / sequence | 56.839 | 55.234 | 6.910 | 7.99x |
| transforms-3840 / cold | 51.931 | 52.862 | 6.903 | 7.66x |
| upstream-karaoke-1920 / same-time | 4.819 | 5.084 | 0.799 | 6.36x |
| upstream-karaoke-1920 / sequence | 4.778 | 5.107 | 0.805 | 6.35x |
| upstream-karaoke-1920 / cold | 31.138 | 31.518 | 1.809 | 17.43x |
