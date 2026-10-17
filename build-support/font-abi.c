/* Compile against the pinned public headers on CI. No font parsing here. */
#include <stddef.h>
#include <stdint.h>
#include <ft2build.h>
#include FT_FREETYPE_H
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
