# Same-runner direct stock libass / Mangetsu API comparison

Commit9265bb3b90b7c17ce07cbf84d60b88ae40ec578f;
[green run40334890325](https://github.com/amanosatosi/taiyou/actions/runs/40334890325),
artifact12655374958. Stock libass f61db567e6593df3470e91594bcd4ad2d0473aff;
Mangetsu db2865d582142717e09160550160b52126aa656b. Same files/fonts/dimensions/
timestamps,31 samples/group, four scenarios, five engine modes:4340 sampled frames.
No process errors, Taiyou candidate/reference RGBA/diagnostic differences, original
Taiyou digest changes or cache OFF/ON default-resource result changes.

Primary external columns are native direct API times, not host RGBA composition:
stock ass_render_frame, Mangetsu ass_render_frame, Mangetsu ass_render_frame_rgba.
Host-owned full-frame RGBA and client composition remain secondary raw columns.
Keep the output-contract distinction visible: Taiyou returns a complete owned
frame; the native APIs return linked bitmap/image regions. That distinction does
not explain the hundreds of milliseconds in Taiyou's changing raster pipeline.

4K changing stress medians: Taiyou1T386.786ms, stock2.072ms, Mangetsu legacy2.901ms,
Mangetsu RGBA24.873ms (Taiyou15.55x RGBA). Warm stress70.005/0.287/1.252/21.130ms.
The raster architecture is now the primary target; no additional MT work. All
distributions, cold setup, seeks, image/area evidence and exact pins are adjacent.
Earlier host-composition datasets remain intact under their original caveats.
