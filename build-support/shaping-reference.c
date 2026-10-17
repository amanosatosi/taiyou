/* Provisional independent C client of pinned HarfBuzz. No Taiyou code is used.
 * Design-unit numeric observations require review; this is not a trusted gate. */
#include <hb.h>
#include <hb-ot.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
static char *read_file(const char *path, size_t cap, size_t *size) {
    FILE *f = fopen(path, "rb");
    if (!f || fseek(f, 0, SEEK_END)) exit(2);
    long n = ftell(f);
    if (n < 0 || (size_t)n > cap || fseek(f, 0, SEEK_SET)) exit(2);
    char *bytes = malloc((size_t)n + 1);
    if (!bytes || fread(bytes, 1, (size_t)n, f) != (size_t)n) exit(2);
    fclose(f); bytes[n] = 0; *size = (size_t)n; return bytes;
}
int main(int argc, char **argv) {
    if (argc != 10) return 2;
    unsigned major, minor, patch; hb_version(&major, &minor, &patch);
    if (major != 14 || minor != 5 || patch != 0) return 3;
    size_t font_len, text_len;
    char *font_bytes = read_file(argv[1], 24 * 1024 * 1024, &font_len);
    char *text = read_file(argv[2], 4096, &text_len);
    unsigned face_index = (unsigned)strtoul(argv[3], NULL, 10);
    unsigned start = (unsigned)strtoul(argv[7], NULL, 10);
    unsigned end = (unsigned)strtoul(argv[8], NULL, 10);
    if (start > end || end > text_len) return 2;
    hb_blob_t *blob = hb_blob_create(font_bytes, (unsigned)font_len, HB_MEMORY_MODE_READONLY, NULL, NULL);
    if (face_index >= hb_face_count(blob)) return 3;
    hb_face_t *face = hb_face_create(blob, face_index);
    hb_font_t *font = hb_font_create(face);
    unsigned upem = hb_face_get_upem(face);
    if (!upem || !hb_face_get_glyph_count(face)) return 3;
    hb_ot_font_set_funcs(font); hb_font_set_scale(font, (int)upem, (int)upem);
    hb_font_extents_t metrics;
    if (!hb_font_get_h_extents(font, &metrics)) return 3;
    hb_buffer_t *buffer = hb_buffer_create();
    hb_buffer_set_direction(buffer, hb_direction_from_string(argv[4], -1));
    hb_buffer_set_script(buffer, hb_script_from_string(argv[5], -1));
    hb_buffer_set_language(buffer, hb_language_from_string(argv[6], -1));
    hb_buffer_set_cluster_level(buffer, HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES);
    hb_buffer_set_flags(buffer, (hb_buffer_flags_t)((start == 0 ? HB_BUFFER_FLAG_BOT : 0) | (end == text_len ? HB_BUFFER_FLAG_EOT : 0)));
    hb_buffer_add_utf8(buffer, text, (int)text_len, start, (int)(end-start));
    hb_feature_t features[3] = {{HB_TAG('k','e','r','n'),1,0,UINT32_MAX}, {HB_TAG('l','i','g','a'),0,0,UINT32_MAX}, {HB_TAG('c','l','i','g'),0,0,UINT32_MAX}};
    features[1].value = features[2].value = (unsigned)atoi(argv[9]);
    const char *shapers[] = {"ot", NULL};
    if (!hb_shape_full(font, buffer, features, 3, shapers) || !hb_buffer_allocation_successful(buffer)) return 3;
    unsigned count, positions_count;
    hb_glyph_info_t *info = hb_buffer_get_glyph_infos(buffer, &count);
    hb_glyph_position_t *pos = hb_buffer_get_glyph_positions(buffer, &positions_count);
    if (count > 10000 || count != positions_count) return 3;
    printf("{\"upem\":%u,\"metrics\":[%d,%d,%d],\"glyphs\":[", upem, metrics.ascender, metrics.descender, metrics.line_gap);
    for (unsigned i=0; i<count; i++) {
        if (i) printf(",");
        printf("[%u,%u,%d,%d,%d,%d]", info[i].codepoint, info[i].cluster, pos[i].x_advance, pos[i].y_advance, pos[i].x_offset, pos[i].y_offset);
    }
    puts("]}");
    hb_buffer_destroy(buffer); hb_font_destroy(font); hb_face_destroy(face); hb_blob_destroy(blob);
    free(text); free(font_bytes); return 0;
}
