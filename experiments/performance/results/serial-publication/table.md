| Workload / state | Taiyou1T ms | reference1T ms | libass+RGBA ms | 1T/libass |
|---|---:|---:|---:|---:|
| dialogue-1920 / same-time | 6.077 | 6.559 | 0.959 | 6.34x |
| dialogue-1920 / sequence | 6.029 | 6.507 | 1.007 | 5.99x |
| dialogue-1920 / cold | 14.251 | 16.045 | 2.829 | 5.04x |
| thick-borders-1920 / same-time | 10.988 | 9.411 | 2.947 | 3.73x |
| thick-borders-1920 / sequence | 11.034 | 9.421 | 3.021 | 3.65x |
| thick-borders-1920 / cold | 53.111 | 52.857 | 5.032 | 10.55x |
| dialogue-3840 / same-time | 18.902 | 20.769 | 4.524 | 4.18x |
| dialogue-3840 / sequence | 18.909 | 21.515 | 4.698 | 4.02x |
| dialogue-3840 / cold | 46.980 | 53.607 | 7.019 | 6.69x |
| combined-abuse-3840 / same-time | 73.220 | 80.012 | 93.409 | 0.78x |
| combined-abuse-3840 / sequence | 401.223 | 409.775 | 101.281 | 3.96x |
| combined-abuse-3840 / cold | 389.518 | 399.769 | 97.031 | 4.01x |
| large-blur-3840 / same-time | 22.624 | 26.036 | 10.236 | 2.21x |
| large-blur-3840 / sequence | 22.809 | 25.458 | 10.402 | 2.19x |
| large-blur-3840 / cold | 122.759 | 127.157 | 13.340 | 9.20x |
| transforms-3840 / same-time | 23.856 | 26.432 | 11.693 | 2.04x |
| transforms-3840 / sequence | 59.903 | 69.064 | 13.905 | 4.31x |
| transforms-3840 / cold | 59.079 | 63.993 | 14.163 | 4.17x |
| upstream-karaoke-1920 / same-time | 5.650 | 6.223 | 1.360 | 4.15x |
| upstream-karaoke-1920 / sequence | 5.622 | 5.867 | 1.373 | 4.09x |
| upstream-karaoke-1920 / cold | 33.240 | 34.998 | 3.457 | 9.61x |
