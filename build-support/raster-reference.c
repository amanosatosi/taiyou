/* CI-only independent bitmap-API cross-check. Not Taiyou production code and
 * not a trusted external validation gate. No custom scan conversion or font parser. */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static long floor_q6(long v) {
    return v >= 0 ? v / 64 : -(long)((-(int64_t)v + 63) / 64);
}
static int bitmap(FT_Library library, FT_Outline *outline, const char *output,
                  const char *metadata, int dx, int dy) {
    long x0 = LONG_MAX, y0 = LONG_MAX, x1 = LONG_MIN, y1 = LONG_MIN;
    unsigned char *bytes = NULL;
    int result = 1;
    FILE *file = NULL;
    for (unsigned i = 0; i < (unsigned)outline->n_points; ++i) {
        long x = outline->points[i].x, y = outline->points[i].y;
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        if (y > y1) y1 = y;
    }
    x0 = floor_q6(x0); y0 = floor_q6(y0);
    x1 = -floor_q6(-x1); y1 = -floor_q6(-y1);
    if (x1 <= x0 || y1 <= y0 || x1 - x0 > 16384 || y1 - y0 > 16384) return 2;
    unsigned width = (unsigned)(x1 - x0), height = (unsigned)(y1 - y0);
    if ((uint64_t)width * height > 2000000) return 2;
    for (unsigned i = 0; i < (unsigned)outline->n_points; ++i) {
        outline->points[i].x -= x0 * 64;
        outline->points[i].y = y1 * 64 - outline->points[i].y;
    }
    bytes = calloc((size_t)width * height, 1);
    if (!bytes) return 2;
    FT_Bitmap target;
    memset(&target, 0, sizeof target);
    target.width = width; target.rows = height; target.pitch = (int)width;
    target.buffer = bytes; target.num_grays = 256; target.pixel_mode = FT_PIXEL_MODE_GRAY;
    if (FT_Outline_Check(outline) || FT_Outline_Get_Bitmap(library, outline, &target)) goto done;
    if (!output) {
        printf("INDEPENDENT_FT_RECT dx=%d dy=%d bounds=%ld,%ld,%ld,%ld bytes=", dx, dy, x0, y0, x1, y1);
        for (size_t i = 0; i < (size_t)width * height; ++i) printf("%s%u", i ? "," : "", bytes[i]);
        puts(""); result = 0; goto done;
    }
    file = fopen(output, "wb");
    if (!file || fwrite(bytes, 1, (size_t)width * height, file) != (size_t)width * height) goto done;
    if (fclose(file)) { file = NULL; goto done; } file = NULL;
    file = fopen(metadata, "w");
    if (!file || fprintf(file, "%u %u %ld %ld\n", width, height, x0, y0) < 0) goto done;
    result = 0;
done:
    if (file && fclose(file)) result = 1;
    free(bytes);
    return result;
}
int main(int argc, char **argv) {
    FT_Library library = NULL;
    FT_Outline outline;
    memset(&outline, 0, sizeof outline);
    FILE *file = NULL;
    int result = 2, allocated = 0;
    if (FT_Init_FreeType(&library)) return 2;
    int major, minor, patch;
    FT_Library_Version(library, &major, &minor, &patch);
    if (major != 2 || minor != 14 || patch != 3) goto done;
    if (argc == 2 && !strcmp(argv[1], "--rects")) {
        const int phases[][2] = {{0,0},{32,0},{0,32},{32,32},{-1,0},{0,-1},{16,0},{0,16},{-32,-32}};
        if (FT_Outline_New(library, 4, 1, &outline)) goto done;
        allocated = 1;
        for (unsigned i = 0; i < 4; ++i) outline.tags[i] = 1;
        outline.contours[0] = 3;
        result = 0;
        for (unsigned n = 0; n < sizeof phases / sizeof phases[0]; ++n) {
            int x = phases[n][0], y = phases[n][1];
            outline.points[0] = (FT_Vector){x,y}; outline.points[1] = (FT_Vector){x+64,y};
            outline.points[2] = (FT_Vector){x+64,y+64}; outline.points[3] = (FT_Vector){x,y+64};
            if (bitmap(library, &outline, NULL, NULL, x, y)) { result = 2; break; }
        }
        goto done;
    }
    if (argc != 4) goto done;
    file = fopen(argv[1], "r");
    if (!file || FT_Outline_New(library, 32767, 32767, &outline)) goto done;
    allocated = 1; outline.n_points = 0; outline.n_contours = 0;
    unsigned points = 0, contours = 0;
    int open = 0;
    char command;
    while (fscanf(file, " %c", &command) == 1) {
        unsigned count;
        if (command == 'Z') {
            if (!open || !points || contours >= 32767) goto done;
            outline.contours[contours++] = (short)(points - 1); open = 0; continue;
        }
        if (command == 'M') { if (open) goto done; open = 1; count = 1; }
        else if (command == 'L') count = 1;
        else if (command == 'Q') count = 2;
        else if (command == 'C') count = 3;
        else goto done;
        if (!open || points + count > 32767) goto done;
        for (unsigned i = 0; i < count; ++i) {
            int64_t x, y;
            if (fscanf(file, " %" SCNd64 " %" SCNd64, &x, &y) != 2 || x < INT32_MIN || x > INT32_MAX || y < INT32_MIN || y > INT32_MAX) goto done;
            outline.points[points] = (FT_Vector){(FT_Pos)x,(FT_Pos)y};
            outline.tags[points++] = (char)(i + 1 == count ? 1 : command == 'Q' ? 0 : 2);
        }
    }
    if (ferror(file) || open || !points || !contours) goto done;
    outline.n_points = (short)points; outline.n_contours = (short)contours;
    result = bitmap(library, &outline, argv[2], argv[3], 0, 0);
done:
    if (file) fclose(file);
    if (allocated) FT_Outline_Done(library, &outline);
    FT_Done_FreeType(library);
    return result;
}
