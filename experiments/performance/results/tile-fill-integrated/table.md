| Workload / state | Taiyou1T ms | reference1T ms | libass ass_render_frame ms | Mangetsu ass_render_frame ms | Mangetsu ass_render_frame_rgba ms | Taiyou/libass | Taiyou/Mangetsu | Taiyou/Mangetsu-RGBA |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dialogue-1920 / same-time | 3.993 | 4.067 | 0.077 | 0.270 | 0.305 | 52.18x | 14.78x | 13.10x |
| dialogue-1920 / sequence | 4.074 | 4.119 | 0.069 | 0.249 | 0.303 | 58.65x | 16.34x | 13.43x |
| dialogue-1920 / cold | 12.330 | 12.481 | 0.662 | 1.356 | 1.413 | 18.62x | 9.10x | 8.73x |
| thick-borders-1920 / same-time | 8.738 | 8.704 | 0.122 | 0.497 | 0.654 | 71.83x | 17.59x | 13.35x |
| thick-borders-1920 / sequence | 8.722 | 8.784 | 0.121 | 0.457 | 0.593 | 72.00x | 19.07x | 14.70x |
| thick-borders-1920 / cold | 51.087 | 51.154 | 0.915 | 1.920 | 1.967 | 55.83x | 26.61x | 25.97x |
| dialogue-3840 / same-time | 10.525 | 10.633 | 0.080 | 0.263 | 0.397 | 132.32x | 40.01x | 26.49x |
| dialogue-3840 / sequence | 10.486 | 10.569 | 0.080 | 0.266 | 0.395 | 131.89x | 39.47x | 26.55x |
| dialogue-3840 / cold | 37.903 | 38.701 | 0.817 | 1.557 | 1.671 | 46.42x | 24.35x | 22.69x |
| combined-abuse-3840 / same-time | 63.052 | 63.195 | 0.288 | 1.266 | 20.942 | 218.77x | 49.81x | 3.01x |
| combined-abuse-3840 / sequence | 388.484 | 394.543 | 2.100 | 3.098 | 24.135 | 184.98x | 125.40x | 16.10x |
| combined-abuse-3840 / cold | 370.080 | 369.711 | 2.309 | 3.993 | 24.203 | 160.29x | 92.69x | 15.29x |
| large-blur-3840 / same-time | 14.308 | 14.441 | 0.111 | 0.378 | 0.854 | 128.53x | 37.87x | 16.76x |
| large-blur-3840 / sequence | 14.382 | 14.421 | 0.107 | 0.378 | 0.880 | 134.91x | 38.07x | 16.34x |
| large-blur-3840 / cold | 107.352 | 108.116 | 1.150 | 1.968 | 2.553 | 93.38x | 54.56x | 42.05x |
| transforms-3840 / same-time | 15.412 | 15.451 | 0.105 | 0.404 | 0.940 | 147.07x | 38.17x | 16.39x |
| transforms-3840 / sequence | 51.232 | 51.642 | 0.896 | 1.172 | 1.779 | 57.19x | 43.72x | 28.79x |
| transforms-3840 / cold | 49.035 | 49.947 | 1.134 | 2.005 | 2.498 | 43.23x | 24.46x | 19.63x |
| upstream-karaoke-1920 / same-time | 3.545 | 3.563 | 0.066 | 0.213 | 0.285 | 53.84x | 16.67x | 12.46x |
| upstream-karaoke-1920 / sequence | 3.512 | 3.453 | 0.062 | 0.194 | 0.261 | 56.69x | 18.11x | 13.46x |
| upstream-karaoke-1920 / cold | 30.618 | 31.240 | 0.801 | 1.264 | 1.350 | 38.25x | 24.22x | 22.69x |
