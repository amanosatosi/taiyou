| Workload / state | Taiyou1T ms | reference1T ms | libass ass_render_frame ms | Mangetsu ass_render_frame ms | Mangetsu ass_render_frame_rgba ms | Taiyou/libass | Taiyou/Mangetsu | Taiyou/Mangetsu-RGBA |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dialogue-1920 / same-time | 3.831 | 3.882 | 0.062 | 0.196 | 0.263 | 61.41x | 19.55x | 14.57x |
| dialogue-1920 / sequence | 3.850 | 3.893 | 0.037 | 0.156 | 0.203 | 103.70x | 24.66x | 18.98x |
| dialogue-1920 / cold | 8.518 | 12.416 | 0.638 | 1.358 | 1.397 | 13.35x | 6.27x | 6.10x |
| thick-borders-1920 / same-time | 8.404 | 8.387 | 0.107 | 0.361 | 0.515 | 78.58x | 23.30x | 16.32x |
| thick-borders-1920 / sequence | 8.669 | 8.611 | 0.120 | 0.363 | 0.603 | 72.48x | 23.87x | 14.38x |
| thick-borders-1920 / cold | 14.452 | 50.271 | 0.893 | 1.727 | 1.882 | 16.19x | 8.37x | 7.68x |
| dialogue-3840 / same-time | 10.125 | 10.330 | 0.075 | 0.248 | 0.381 | 134.75x | 40.79x | 26.57x |
| dialogue-3840 / sequence | 10.295 | 10.252 | 0.075 | 0.258 | 0.388 | 137.18x | 39.98x | 26.56x |
| dialogue-3840 / cold | 22.075 | 38.459 | 0.808 | 1.529 | 1.652 | 27.32x | 14.43x | 13.36x |
| combined-abuse-3840 / same-time | 62.827 | 62.696 | 0.280 | 1.260 | 20.748 | 224.12x | 49.87x | 3.03x |
| combined-abuse-3840 / sequence | 139.503 | 377.125 | 2.054 | 2.940 | 24.577 | 67.93x | 47.46x | 5.68x |
| combined-abuse-3840 / cold | 133.753 | 360.735 | 2.320 | 3.879 | 24.005 | 57.65x | 34.48x | 5.57x |
| large-blur-3840 / same-time | 14.108 | 14.278 | 0.105 | 0.369 | 0.876 | 134.56x | 38.26x | 16.10x |
| large-blur-3840 / sequence | 14.129 | 14.170 | 0.103 | 0.368 | 0.885 | 136.94x | 38.35x | 15.97x |
| large-blur-3840 / cold | 46.388 | 107.369 | 1.150 | 2.045 | 2.434 | 40.35x | 22.69x | 19.06x |
| transforms-3840 / same-time | 15.211 | 15.226 | 0.104 | 0.423 | 0.993 | 145.59x | 35.95x | 15.32x |
| transforms-3840 / sequence | 32.690 | 51.731 | 0.828 | 1.140 | 1.722 | 39.50x | 28.68x | 18.98x |
| transforms-3840 / cold | 30.695 | 49.166 | 1.123 | 1.985 | 2.479 | 27.33x | 15.46x | 12.38x |
| upstream-karaoke-1920 / same-time | 3.372 | 3.413 | 0.037 | 0.121 | 0.193 | 92.31x | 27.87x | 17.50x |
| upstream-karaoke-1920 / sequence | 3.416 | 3.421 | 0.041 | 0.151 | 0.198 | 84.15x | 22.57x | 17.29x |
| upstream-karaoke-1920 / cold | 13.722 | 31.042 | 0.760 | 1.209 | 1.284 | 18.04x | 11.35x | 10.69x |
