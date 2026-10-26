| Workload / state | Taiyou1T ms | Taiyou4T ms | libass+RGBA ms | MT/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 6.776 | 7.155 | 0.397 | 18.03x |
| dialogue-1920 / sequence | 6.634 | 7.047 | 0.364 | 19.37x |
| dialogue-1920 / cold | 18.145 | 18.564 | 2.167 | 8.57x |
| thick-borders-1920 / same-time | 12.096 | 12.493 | 1.055 | 11.84x |
| thick-borders-1920 / sequence | 12.072 | 12.492 | 1.071 | 11.66x |
| thick-borders-1920 / cold | 64.627 | 65.147 | 3.099 | 21.02x |
| dialogue-3840 / same-time | 22.806 | 23.516 | 2.371 | 9.92x |
| dialogue-3840 / sequence | 22.987 | 25.006 | 2.405 | 10.40x |
| dialogue-3840 / cold | 62.588 | 61.885 | 4.457 | 13.88x |
| combined-abuse-3840 / same-time | 228.519 | 123.881 | 56.532 | 2.19x |
| combined-abuse-3840 / sequence | 589.293 | 485.134 | 60.736 | 7.99x |
| combined-abuse-3840 / cold | 578.476 | 471.700 | 59.198 | 7.97x |
| large-blur-3840 / same-time | 33.155 | 33.252 | 4.508 | 7.38x |
| large-blur-3840 / sequence | 33.828 | 33.473 | 4.503 | 7.43x |
| large-blur-3840 / cold | 152.169 | 152.990 | 6.621 | 23.11x |
| transforms-3840 / same-time | 36.600 | 37.829 | 3.754 | 10.08x |
| transforms-3840 / sequence | 87.992 | 90.107 | 4.470 | 20.16x |
| transforms-3840 / cold | 83.553 | 84.660 | 5.645 | 15.00x |
| upstream-karaoke-1920 / same-time | 7.088 | 7.155 | 1.417 | 5.05x |
| upstream-karaoke-1920 / sequence | 7.767 | 7.277 | 1.434 | 5.07x |
| upstream-karaoke-1920 / cold | 43.187 | 43.480 | 3.454 | 12.59x |
