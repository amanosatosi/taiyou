| Workload / state | Taiyou1T ms | reference1T ms | libass ass_render_frame ms | Mangetsu ass_render_frame ms | Mangetsu ass_render_frame_rgba ms | Taiyou/libass | Taiyou/Mangetsu | Taiyou/Mangetsu-RGBA |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dialogue-1920 / same-time | 3.763 | 3.808 | 0.031 | 0.123 | 0.156 | 120.96x | 30.66x | 24.10x |
| dialogue-1920 / sequence | 3.760 | 3.796 | 0.029 | 0.126 | 0.161 | 131.18x | 29.75x | 23.38x |
| dialogue-1920 / cold | 11.965 | 11.872 | 0.585 | 1.140 | 1.303 | 20.47x | 10.49x | 9.18x |
| thick-borders-1920 / same-time | 8.203 | 8.366 | 0.076 | 0.296 | 0.442 | 107.47x | 27.73x | 18.54x |
| thick-borders-1920 / sequence | 8.252 | 8.384 | 0.077 | 0.294 | 0.438 | 107.26x | 28.08x | 18.86x |
| thick-borders-1920 / cold | 48.463 | 47.968 | 0.829 | 1.588 | 1.727 | 58.45x | 30.51x | 28.07x |
| dialogue-3840 / same-time | 9.823 | 9.843 | 0.065 | 0.240 | 0.358 | 150.09x | 41.00x | 27.44x |
| dialogue-3840 / sequence | 9.797 | 9.819 | 0.068 | 0.237 | 0.369 | 143.41x | 41.37x | 26.57x |
| dialogue-3840 / cold | 37.000 | 36.959 | 0.787 | 1.515 | 1.636 | 47.03x | 24.42x | 22.62x |
| combined-abuse-3840 / same-time | 62.341 | 62.353 | 0.269 | 1.247 | 21.051 | 231.45x | 49.99x | 2.96x |
| combined-abuse-3840 / sequence | 374.172 | 374.108 | 2.005 | 2.958 | 24.255 | 186.63x | 126.49x | 15.43x |
| combined-abuse-3840 / cold | 362.259 | 361.360 | 2.284 | 3.335 | 23.690 | 158.59x | 108.64x | 15.29x |
| large-blur-3840 / same-time | 13.676 | 13.694 | 0.093 | 0.344 | 0.839 | 146.78x | 39.75x | 16.30x |
| large-blur-3840 / sequence | 13.662 | 13.721 | 0.090 | 0.334 | 0.841 | 151.24x | 40.84x | 16.25x |
| large-blur-3840 / cold | 107.211 | 106.096 | 1.121 | 2.018 | 2.411 | 95.62x | 53.12x | 44.47x |
| transforms-3840 / same-time | 14.697 | 14.728 | 0.094 | 0.356 | 0.921 | 155.70x | 41.25x | 15.96x |
| transforms-3840 / sequence | 49.870 | 49.565 | 0.790 | 1.067 | 1.680 | 63.12x | 46.74x | 29.68x |
| transforms-3840 / cold | 48.165 | 48.111 | 1.065 | 1.919 | 2.477 | 45.23x | 25.11x | 19.44x |
| upstream-karaoke-1920 / same-time | 3.382 | 3.359 | 0.030 | 0.118 | 0.182 | 114.56x | 28.78x | 18.59x |
| upstream-karaoke-1920 / sequence | 3.441 | 3.392 | 0.030 | 0.110 | 0.186 | 116.54x | 31.25x | 18.54x |
| upstream-karaoke-1920 / cold | 30.730 | 30.253 | 0.740 | 1.152 | 1.217 | 41.54x | 26.68x | 25.24x |
