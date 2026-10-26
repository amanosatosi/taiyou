| Workload / state | Taiyou1T ms | reference1T ms | libass ass_render_frame ms | Mangetsu ass_render_frame ms | Mangetsu ass_render_frame_rgba ms | Taiyou/libass | Taiyou/Mangetsu | Taiyou/Mangetsu-RGBA |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dialogue-1920 / same-time | 3.897 | 5.958 | 0.049 | 0.138 | 0.189 | 79.23x | 28.27x | 20.59x |
| dialogue-1920 / sequence | 3.919 | 5.455 | 0.071 | 0.200 | 0.213 | 55.12x | 19.62x | 18.37x |
| dialogue-1920 / cold | 12.692 | 16.468 | 0.617 | 1.305 | 1.476 | 20.57x | 9.73x | 8.60x |
| thick-borders-1920 / same-time | 8.351 | 8.614 | 0.106 | 0.337 | 0.465 | 78.47x | 24.77x | 17.98x |
| thick-borders-1920 / sequence | 8.321 | 8.673 | 0.099 | 0.313 | 0.466 | 84.03x | 26.61x | 17.85x |
| thick-borders-1920 / cold | 51.895 | 51.988 | 0.933 | 1.861 | 1.926 | 55.63x | 27.88x | 26.95x |
| dialogue-3840 / same-time | 10.367 | 14.999 | 0.077 | 0.214 | 0.367 | 134.35x | 48.43x | 28.26x |
| dialogue-3840 / sequence | 10.394 | 15.192 | 0.082 | 0.220 | 0.364 | 126.13x | 47.28x | 28.57x |
| dialogue-3840 / cold | 39.943 | 48.110 | 0.869 | 1.691 | 1.838 | 45.99x | 23.63x | 21.73x |
| combined-abuse-3840 / same-time | 70.005 | 77.795 | 0.287 | 1.252 | 21.130 | 243.71x | 55.90x | 3.31x |
| combined-abuse-3840 / sequence | 386.786 | 389.146 | 2.072 | 2.901 | 24.873 | 186.65x | 133.31x | 15.55x |
| combined-abuse-3840 / cold | 370.603 | 375.600 | 2.375 | 3.829 | 24.978 | 156.06x | 96.80x | 14.84x |
| large-blur-3840 / same-time | 14.421 | 19.541 | 0.111 | 0.329 | 0.858 | 130.42x | 43.82x | 16.80x |
| large-blur-3840 / sequence | 14.436 | 19.330 | 0.110 | 0.333 | 0.863 | 131.43x | 43.41x | 16.72x |
| large-blur-3840 / cold | 110.214 | 117.225 | 1.213 | 2.175 | 2.729 | 90.86x | 50.68x | 40.39x |
| transforms-3840 / same-time | 15.867 | 20.730 | 0.112 | 0.347 | 0.942 | 141.23x | 45.70x | 16.85x |
| transforms-3840 / sequence | 54.103 | 63.180 | 0.876 | 1.110 | 1.758 | 61.79x | 48.72x | 30.78x |
| transforms-3840 / cold | 52.196 | 60.688 | 1.184 | 2.151 | 2.652 | 44.08x | 24.27x | 19.68x |
| upstream-karaoke-1920 / same-time | 3.603 | 5.865 | 0.057 | 0.162 | 0.251 | 63.23x | 22.23x | 14.36x |
| upstream-karaoke-1920 / sequence | 3.656 | 5.979 | 0.071 | 0.185 | 0.262 | 51.24x | 19.73x | 13.97x |
| upstream-karaoke-1920 / cold | 32.924 | 37.579 | 0.817 | 1.149 | 1.467 | 40.31x | 28.65x | 22.44x |
