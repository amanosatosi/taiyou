/* T015 test bridge only. Decode with the original pinned libass image.c;
 * emit its pixels as big-endian RGBA16 with ordinary (uninverted) alpha.
 * No normalization arithmetic is reimplemented in this bridge. */
#include "image.h"
#include <stdio.h>

int main(int argc, char **argv)
{
    Image16 image = {0};
    if (argc != 2 || !read_png(argv[1], &image))
        return 1;
    if (image.width <= 0 || image.height <= 0 ||
        (uint64_t) image.width * image.height > 16 * 1024 * 1024) {
        free(image.buffer);
        return 2;
    }
    size_t count = (size_t) image.width * image.height * 4;
    for (size_t i = 0; i < count; i++) {
        uint16_t value = i % 4 == 3 ? (uint16_t) ~image.buffer[i] : image.buffer[i];
        if (putchar(value >> 8) == EOF || putchar(value & 255) == EOF) {
            free(image.buffer);
            return 3;
        }
    }
    free(image.buffer);
    return 0;
}
