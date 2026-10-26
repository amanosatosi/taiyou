| Workload / state | Taiyou1T ms | frozen1T ms | libass+RGBA ms | 1T/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 4.964 | 5.316 | 0.722 | 6.88x |
| dialogue-1920 / sequence | 4.997 | 5.306 | 0.702 | 7.12x |
| dialogue-1920 / cold | 14.722 | 15.053 | 1.982 | 7.43x |
| thick-borders-1920 / same-time | 7.111 | 9.005 | 2.459 | 2.89x |
| thick-borders-1920 / sequence | 7.124 | 8.768 | 2.429 | 2.93x |
| thick-borders-1920 / cold | 51.568 | 53.193 | 3.878 | 13.30x |
| dialogue-3840 / same-time | 12.955 | 14.258 | 3.216 | 4.03x |
| dialogue-3840 / sequence | 12.670 | 14.195 | 3.188 | 3.97x |
| dialogue-3840 / cold | 45.216 | 46.092 | 4.650 | 9.72x |
| combined-abuse-3840 / same-time | 60.092 | 157.377 | 80.058 | 0.75x |
| combined-abuse-3840 / sequence | 350.320 | 452.143 | 86.492 | 4.05x |
| combined-abuse-3840 / cold | 344.596 | 442.826 | 82.755 | 4.16x |
| large-blur-3840 / same-time | 15.281 | 20.008 | 8.030 | 1.90x |
| large-blur-3840 / sequence | 16.342 | 20.256 | 8.004 | 2.04x |
| large-blur-3840 / cold | 111.930 | 116.790 | 9.758 | 11.47x |
| transforms-3840 / same-time | 17.076 | 22.590 | 9.074 | 1.88x |
| transforms-3840 / sequence | 58.482 | 65.158 | 10.794 | 5.42x |
| transforms-3840 / cold | 55.440 | 62.059 | 10.810 | 5.13x |
| upstream-karaoke-1920 / same-time | 4.299 | 4.991 | 1.033 | 4.16x |
| upstream-karaoke-1920 / sequence | 4.310 | 5.013 | 1.034 | 4.17x |
| upstream-karaoke-1920 / cold | 35.080 | 37.025 | 2.382 | 14.72x |
