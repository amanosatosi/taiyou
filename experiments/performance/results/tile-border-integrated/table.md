| Workload / state | Taiyou1T ms | reference1T ms | libass ass_render_frame ms | Mangetsu ass_render_frame ms | Mangetsu ass_render_frame_rgba ms | Taiyou/libass | Taiyou/Mangetsu | Taiyou/Mangetsu-RGBA |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dialogue-1920 / same-time | 4.001 | 3.846 | 0.062 | 0.237 | 0.256 | 64.12x | 16.86x | 15.62x |
| dialogue-1920 / sequence | 3.904 | 3.840 | 0.048 | 0.165 | 0.223 | 82.00x | 23.71x | 17.50x |
| dialogue-1920 / cold | 13.895 | 12.347 | 0.639 | 1.337 | 1.338 | 21.74x | 10.39x | 10.38x |
| thick-borders-1920 / same-time | 8.473 | 8.526 | 0.104 | 0.429 | 0.539 | 81.74x | 19.73x | 15.71x |
| thick-borders-1920 / sequence | 8.446 | 8.338 | 0.086 | 0.380 | 0.501 | 98.72x | 22.21x | 16.85x |
| thick-borders-1920 / cold | 22.525 | 49.937 | 0.875 | 1.769 | 1.841 | 25.73x | 12.73x | 12.24x |
| dialogue-3840 / same-time | 10.334 | 10.323 | 0.075 | 0.247 | 0.384 | 137.24x | 41.88x | 26.89x |
| dialogue-3840 / sequence | 10.415 | 10.440 | 0.075 | 0.253 | 0.387 | 138.66x | 41.16x | 26.90x |
| dialogue-3840 / cold | 47.440 | 38.417 | 0.818 | 1.518 | 1.630 | 57.98x | 31.25x | 29.11x |
| combined-abuse-3840 / same-time | 66.107 | 63.345 | 0.282 | 1.203 | 20.720 | 234.72x | 54.96x | 3.19x |
| combined-abuse-3840 / sequence | 214.673 | 376.834 | 2.118 | 2.931 | 24.012 | 101.37x | 73.24x | 8.94x |
| combined-abuse-3840 / cold | 212.406 | 361.460 | 2.306 | 3.810 | 23.565 | 92.10x | 55.76x | 9.01x |
| large-blur-3840 / same-time | 14.307 | 14.239 | 0.102 | 0.383 | 0.903 | 139.85x | 37.32x | 15.85x |
| large-blur-3840 / sequence | 14.164 | 14.285 | 0.101 | 0.389 | 0.870 | 139.60x | 36.39x | 16.28x |
| large-blur-3840 / cold | 66.478 | 107.997 | 1.158 | 1.955 | 2.071 | 57.42x | 34.00x | 32.10x |
| transforms-3840 / same-time | 15.354 | 15.212 | 0.105 | 0.418 | 0.929 | 146.20x | 36.72x | 16.53x |
| transforms-3840 / sequence | 63.158 | 50.601 | 0.830 | 1.182 | 1.759 | 76.05x | 53.44x | 35.90x |
| transforms-3840 / cold | 60.747 | 49.525 | 1.127 | 1.990 | 2.118 | 53.92x | 30.52x | 28.68x |
| upstream-karaoke-1920 / same-time | 3.401 | 3.497 | 0.041 | 0.151 | 0.203 | 83.90x | 22.50x | 16.73x |
| upstream-karaoke-1920 / sequence | 3.420 | 3.890 | 0.038 | 0.132 | 0.193 | 89.94x | 25.92x | 17.68x |
| upstream-karaoke-1920 / cold | 24.483 | 30.860 | 0.764 | 1.237 | 1.324 | 32.04x | 19.80x | 18.50x |
