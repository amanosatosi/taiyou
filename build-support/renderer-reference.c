/* CI-only independently authored single-H pixel expectation. Uses the pinned
 * FreeType bitmap API, not Taiyou layout/geometry/raster/paint functions. This
 * candidate-side reference is not a trusted T005 oracle or approved golden. */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void require(int ok) { if (!ok) { fputs("renderer reference failure\n", stderr); exit(1); } }
static long quantize(double n) { require(isfinite(n) && fabs(n) < 2000000000.0); return (long)round(n); }
int main(int argc, char **argv) {
    FT_Library library; FT_Face face; int major, minor, patch;
    require(argc == 3 && FT_Init_FreeType(&library) == 0);
    FT_Library_Version(library, &major, &minor, &patch);
    require(major == 2 && minor == 14 && patch == 3);
    require(FT_New_Face(library, argv[1], 0, &face) == 0 && face->units_per_EM > 0);
    double scale = 20.0 / face->units_per_EM;
    double ascent = face->ascender * scale, descent = -face->descender * scale;
    printf("INDEPENDENT_H_METRICS upem=%u ascender=%ld descender=%ld\n", face->units_per_EM, (long)face->ascender, (long)face->descender);
    for (int a = 1; a <= 10; a++) {
        require(FT_Load_Char(face, 'H', FT_LOAD_NO_SCALE | FT_LOAD_NO_HINTING | FT_LOAD_NO_BITMAP | FT_LOAD_IGNORE_TRANSFORM) == 0);
        require(face->glyph->format == FT_GLYPH_FORMAT_OUTLINE);
        FT_Outline *outline = &face->glyph->outline;
        double advance = face->glyph->metrics.horiAdvance * scale;
        int col = (a - 1) % 3, row = (a - 1) / 3;
        double x = col == 0 ? 10.0 : col == 1 ? 80.0 : 150.0;
        double y = row == 0 ? 90.0 : row == 1 ? 50.0 : 10.0;
        x -= col == 0 ? 0.0 : col == 1 ? advance / 2.0 : advance;
        y += row == 0 ? -descent : row == 1 ? (ascent - descent) / 2.0 : ascent;
        if (a == 10) { x = -2.0; y = -8.0 + ascent; }
        for (unsigned int i = 0; i < outline->n_points; i++) {
            double px = outline->points[i].x * scale + x;
            double py = -outline->points[i].y * scale + y;
            outline->points[i].x = quantize(px * 64.0);
            outline->points[i].y = 100 * 64 - quantize(py * 64.0);
        }
        unsigned char data[160 * 100] = {0};
        FT_Bitmap bitmap; memset(&bitmap, 0, sizeof(bitmap));
        bitmap.width = 160; bitmap.rows = 100; bitmap.pitch = 160;
        bitmap.buffer = data; bitmap.pixel_mode = FT_PIXEL_MODE_GRAY; bitmap.num_grays = 256;
        require(FT_Outline_Get_Bitmap(library, outline, &bitmap) == 0);
        char path[4096]; int n = snprintf(path, sizeof(path), "%s/align%d.a8", argv[2], a);
        require(n > 0 && (size_t)n < sizeof(path));
        FILE *file = fopen(path, "wb"); require(file != NULL);
        require(fwrite(data, 1, sizeof(data), file) == sizeof(data)); require(fclose(file) == 0);
    }
    require(FT_Done_Face(face) == 0); require(FT_Done_FreeType(library) == 0);
    return 0;
}
