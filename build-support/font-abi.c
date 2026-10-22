/* Compile against the pinned public headers on CI. No font parsing here. */
#include <stddef.h>
#include <stdint.h>
#include <hb.h>
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include FT_SFNT_NAMES_H
#include FT_TRUETYPE_TABLES_H
struct RustSfntName {
    uint16_t platform, encoding, language, name;
    unsigned char *string;
    uint32_t length;
};
_Static_assert(sizeof(struct RustSfntName) == sizeof(FT_SfntName), "SFNT ABI size");
_Static_assert(offsetof(struct RustSfntName, platform) == offsetof(FT_SfntName, platform_id), "platform offset");
_Static_assert(offsetof(struct RustSfntName, encoding) == offsetof(FT_SfntName, encoding_id), "encoding offset");
_Static_assert(offsetof(struct RustSfntName, language) == offsetof(FT_SfntName, language_id), "language offset");
_Static_assert(offsetof(struct RustSfntName, name) == offsetof(FT_SfntName, name_id), "name offset");
_Static_assert(offsetof(struct RustSfntName, string) == offsetof(FT_SfntName, string), "pointer offset");
_Static_assert(offsetof(struct RustSfntName, length) == offsetof(FT_SfntName, string_len), "length offset");
_Static_assert(sizeof(FT_UInt) == sizeof(uint32_t), "FT_UInt width");
_Static_assert(sizeof(FT_Int) == sizeof(int32_t), "FT_Int width");
_Static_assert(offsetof(TT_OS2, usWeightClass) == 4, "OS/2 weight prefix offset");
_Static_assert(FT_SFNT_OS2 == 2, "OS/2 native table selector");
_Static_assert(_Generic(&FT_Load_Sfnt_Table, FT_Error (*)(FT_Face, FT_ULong, FT_Long, FT_Byte *, FT_ULong *): 1, default: 0), "SFNT bounded table read ABI");
_Static_assert(sizeof(hb_glyph_info_t) == 20, "HarfBuzz glyph-info ABI");
_Static_assert(offsetof(hb_glyph_info_t, cluster) == 8, "HarfBuzz cluster offset");
_Static_assert(sizeof(hb_glyph_position_t) == 20, "HarfBuzz positions ABI");
_Static_assert(offsetof(hb_glyph_position_t, y_offset) == 12, "HarfBuzz y offset");
_Static_assert(sizeof(hb_font_extents_t) == 48, "HarfBuzz extents ABI");
_Static_assert(sizeof(hb_glyph_extents_t) == 16, "HarfBuzz glyph extents ABI");
_Static_assert(offsetof(hb_glyph_extents_t, height) == 12, "HarfBuzz glyph height offset");
_Static_assert(sizeof(hb_feature_t) == 16, "HarfBuzz features ABI");
_Static_assert(HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES == 0, "cluster policy");
struct RustOutlineFuncs {
    FT_Outline_MoveToFunc move_to;
    FT_Outline_LineToFunc line_to;
    FT_Outline_ConicToFunc conic_to;
    FT_Outline_CubicToFunc cubic_to;
    FT_Int shift;
    FT_Pos delta;
};
_Static_assert(sizeof(struct RustOutlineFuncs) == sizeof(FT_Outline_Funcs), "outline callback ABI");
_Static_assert(offsetof(struct RustOutlineFuncs, shift) == offsetof(FT_Outline_Funcs, shift), "outline shift offset");
_Static_assert(offsetof(struct RustOutlineFuncs, delta) == offsetof(FT_Outline_Funcs, delta), "outline delta offset");
_Static_assert((FT_LOAD_NO_SCALE | FT_LOAD_NO_HINTING | FT_LOAD_NO_BITMAP | FT_LOAD_IGNORE_TRANSFORM) == 2059, "outline load policy");
struct RustRasterSpan { uint16_t x, len; uint8_t coverage; };
struct RustRasterParams {
    const void *target, *source;
    int flags;
    FT_SpanFunc gray_spans, black_spans;
    FT_Raster_BitTest_Func bit_test;
    FT_Raster_BitSet_Func bit_set;
    void *user;
    FT_BBox clip_box;
};
_Static_assert(sizeof(struct RustRasterSpan) == sizeof(FT_Span), "span ABI");
_Static_assert(offsetof(struct RustRasterSpan, coverage) == offsetof(FT_Span, coverage), "span coverage offset");
_Static_assert(_Generic(((FT_Span*)0)->x, unsigned short: 1, default: 0), "pinned span x is unsigned");
_Static_assert(sizeof(struct RustRasterParams) == sizeof(FT_Raster_Params), "raster params ABI");
_Static_assert(offsetof(struct RustRasterParams, gray_spans) == offsetof(FT_Raster_Params, gray_spans), "gray callback offset");
_Static_assert(offsetof(struct RustRasterParams, user) == offsetof(FT_Raster_Params, user), "raster user offset");
_Static_assert(offsetof(struct RustRasterParams, clip_box) == offsetof(FT_Raster_Params, clip_box), "raster clip offset");
_Static_assert((FT_RASTER_FLAG_AA | FT_RASTER_FLAG_DIRECT | FT_RASTER_FLAG_CLIP) == 7, "raster policy flags");
