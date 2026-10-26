| Workload / state | Taiyou1T ms | Taiyou4T ms | libass+RGBA ms | MT/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 6.327 | 6.545 | 0.337 | 19.40x |
| dialogue-1920 / sequence | 6.364 | 6.795 | 0.336 | 20.23x |
| dialogue-1920 / cold | 16.683 | 17.253 | 1.751 | 9.85x |
| thick-borders-1920 / same-time | 11.463 | 11.979 | 1.002 | 11.95x |
| thick-borders-1920 / sequence | 11.495 | 11.937 | 1.003 | 11.90x |
| thick-borders-1920 / cold | 63.539 | 64.011 | 2.644 | 24.21x |
| dialogue-3840 / same-time | 22.937 | 22.925 | 1.729 | 13.26x |
| dialogue-3840 / sequence | 22.375 | 21.821 | 1.704 | 12.81x |
| dialogue-3840 / cold | 59.776 | 60.917 | 3.462 | 17.60x |
| combined-abuse-3840 / same-time | 224.433 | 119.383 | 55.973 | 2.13x |
| combined-abuse-3840 / sequence | 582.496 | 477.933 | 59.700 | 8.01x |
| combined-abuse-3840 / cold | 569.664 | 465.435 | 58.397 | 7.97x |
| large-blur-3840 / same-time | 32.706 | 32.206 | 4.314 | 7.47x |
| large-blur-3840 / sequence | 31.566 | 31.997 | 4.274 | 7.49x |
| large-blur-3840 / cold | 154.270 | 150.133 | 5.898 | 25.46x |
| transforms-3840 / same-time | 35.081 | 37.658 | 3.367 | 11.18x |
| transforms-3840 / sequence | 86.270 | 89.156 | 3.748 | 23.79x |
| transforms-3840 / cold | 83.251 | 82.904 | 5.279 | 15.71x |
| upstream-karaoke-1920 / same-time | 7.106 | 6.350 | 1.236 | 5.14x |
| upstream-karaoke-1920 / sequence | 6.391 | 6.264 | 1.235 | 5.07x |
| upstream-karaoke-1920 / cold | 42.966 | 42.574 | 2.861 | 14.88x |
