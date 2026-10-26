| Workload / state | Taiyou1T ms | reference1T ms | libass+RGBA ms | 1T/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 6.181 | 6.088 | 0.884 | 6.99x |
| dialogue-1920 / sequence | 6.157 | 6.355 | 0.872 | 7.06x |
| dialogue-1920 / cold | 16.332 | 18.441 | 2.794 | 5.85x |
| thick-borders-1920 / same-time | 9.266 | 9.131 | 2.994 | 3.10x |
| thick-borders-1920 / sequence | 9.256 | 9.166 | 2.933 | 3.16x |
| thick-borders-1920 / cold | 50.983 | 63.625 | 5.093 | 10.01x |
| dialogue-3840 / same-time | 21.958 | 22.398 | 4.484 | 4.90x |
| dialogue-3840 / sequence | 20.546 | 22.168 | 4.446 | 4.62x |
| dialogue-3840 / cold | 53.779 | 60.854 | 6.924 | 7.77x |
| combined-abuse-3840 / same-time | 77.178 | 77.057 | 93.448 | 0.83x |
| combined-abuse-3840 / sequence | 407.447 | 447.059 | 101.493 | 4.01x |
| combined-abuse-3840 / cold | 387.002 | 435.293 | 97.343 | 3.98x |
| large-blur-3840 / same-time | 25.714 | 26.259 | 10.304 | 2.50x |
| large-blur-3840 / sequence | 26.481 | 26.386 | 10.308 | 2.57x |
| large-blur-3840 / cold | 125.162 | 146.967 | 13.227 | 9.46x |
| transforms-3840 / same-time | 25.636 | 25.490 | 11.445 | 2.24x |
| transforms-3840 / sequence | 67.674 | 79.274 | 13.800 | 4.90x |
| transforms-3840 / cold | 64.795 | 74.917 | 14.359 | 4.51x |
| upstream-karaoke-1920 / same-time | 5.398 | 5.971 | 1.266 | 4.26x |
| upstream-karaoke-1920 / sequence | 5.636 | 5.762 | 1.296 | 4.35x |
| upstream-karaoke-1920 / cold | 35.551 | 43.865 | 3.424 | 10.38x |
