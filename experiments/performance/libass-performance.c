/* Actions-only direct renderer benchmark + secondary scalar owned
 * premultiplied RGBA consumer. Stock libass and Mangetsu legacy builds time
 * ass_render_frame(); MANGESTU_RGBA_MODE times ass_render_frame_rgba().
 * The library source is not instrumented. Never reuse the final RGBA frame. */
#define _POSIX_C_SOURCE 200809L
#include <ass/ass.h>
#include <inttypes.h>
#include <limits.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static void fail(const char *message) { fprintf(stderr, "%s\n", message); exit(2); }
static uint64_t clock_ns(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) fail("clock_gettime");
    return (uint64_t)t.tv_sec * UINT64_C(1000000000) + (uint64_t)t.tv_nsec;
}
static char *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (!file || fseek(file, 0, SEEK_END)) fail("input open/seek");
    long size = ftell(file);
    if (size < 0 || size > 16 * 1024 * 1024 || fseek(file, 0, SEEK_SET)) fail("input extent");
    char *data = malloc((size_t)size + 1);
    if (!data || fread(data, 1, (size_t)size, file) != (size_t)size) fail("input read");
    fclose(file); data[size] = 0; *length = (size_t)size; return data;
}
static void messages(int level, const char *format, va_list args, void *opaque) {
    (void)format; (void)args;
    if (level <= 2) (*(unsigned *)opaque)++;
}
static ASS_Renderer *new_renderer(ASS_Library *library, int width, int height) {
    ASS_Renderer *renderer = ass_renderer_init(library);
    if (!renderer) fail("ass_renderer_init");
    ass_set_frame_size(renderer, width, height);
    ass_set_storage_size(renderer, width, height);
    ass_set_fonts(renderer, NULL, "Source Sans 3", ASS_FONTPROVIDER_NONE, NULL, 1);
    /* Normal production shaper, hinting, CPU dispatch and default cache limits. */
    return renderer;
}
static unsigned mul255(unsigned a, unsigned b) { return (a * b + 127) / 255; }
static uint8_t *render_rgba(ASS_Renderer *renderer, ASS_Track *track,
                          long long time_ms, int width, int height, uint64_t *render_time,
                          uint64_t *compose_time, unsigned *images, int *changed) {
    uint64_t start = clock_ns();
#ifdef MANGESTU_RGBA_MODE
#ifndef LIBASSMOD_FEATURE_RGBA
#error "MANGESTU_RGBA_MODE requires the pinned libassmod RGBA API"
#endif
    ASS_ImageRGBA *head = ass_render_frame_rgba(renderer, track, time_ms, changed);
#else
    ASS_Image *head = ass_render_frame(renderer, track, time_ms, changed);
#endif
    *render_time = clock_ns() - start;
    start = clock_ns();
    size_t length = (size_t)width * (size_t)height * 4;
    uint8_t *pixels = malloc(length);
    if (!pixels) fail("RGBA allocation");
    memset(pixels, 0, length);
    *images = 0;
#ifdef MANGESTU_RGBA_MODE
    for (ASS_ImageRGBA *image = head; image; image = image->next) {
        (*images)++;
        int64_t x0 = image->dst_x > 0 ? image->dst_x : 0;
        int64_t y0 = image->dst_y > 0 ? image->dst_y : 0;
        int64_t x1 = (int64_t)image->dst_x + image->w;
        int64_t y1 = (int64_t)image->dst_y + image->h;
        if (x1 > width) x1 = width;
        if (y1 > height) y1 = height;
        if (x0 >= x1 || y0 >= y1) continue;
        if (!image->rgba || image->stride < image->w * 4) fail("invalid ASS_ImageRGBA extent");
        for (int64_t y = y0; y < y1; y++) {
            const uint8_t *source = image->rgba + (size_t)(y - image->dst_y) * (size_t)image->stride +
                                    (size_t)(x0 - image->dst_x) * 4;
            uint8_t *destination = pixels + ((size_t)y * (size_t)width + (size_t)x0) * 4;
            for (int64_t x = x0; x < x1; x++, source += 4, destination += 4) {
                unsigned alpha = source[3];
                for (int channel = 0; channel < 3; channel++)
                    destination[channel] = (uint8_t)(source[channel] + mul255(destination[channel], 255 - alpha));
                destination[3] = (uint8_t)(alpha + mul255(destination[3], 255 - alpha));
            }
        }
    }
    ass_free_images_rgba(head);
#else
    for (ASS_Image *image = head; image; image = image->next) {
        (*images)++;
        int64_t x0 = image->dst_x > 0 ? image->dst_x : 0;
        int64_t y0 = image->dst_y > 0 ? image->dst_y : 0;
        int64_t x1 = (int64_t)image->dst_x + image->w;
        int64_t y1 = (int64_t)image->dst_y + image->h;
        if (x1 > width) x1 = width;
        if (y1 > height) y1 = height;
        if (x0 >= x1 || y0 >= y1) continue;
        if (!image->bitmap || image->stride < image->w) fail("invalid ASS_Image extent");
        unsigned color[3] = {image->color >> 24, (image->color >> 16) & 255, (image->color >> 8) & 255};
        unsigned opacity = 255 - (image->color & 255);
        for (int64_t y = y0; y < y1; y++) {
            const uint8_t *source = image->bitmap + (size_t)(y - image->dst_y) * (size_t)image->stride;
            uint8_t *destination = pixels + ((size_t)y * (size_t)width + (size_t)x0) * 4;
            for (int64_t x = x0; x < x1; x++, destination += 4) {
                unsigned alpha = mul255(source[x - image->dst_x], opacity);
                for (int channel = 0; channel < 3; channel++)
                    destination[channel] = (uint8_t)(mul255(color[channel], alpha) + mul255(destination[channel], 255 - alpha));
                destination[3] = (uint8_t)(alpha + mul255(destination[3], 255 - alpha));
            }
        }
    }
#endif
    *compose_time = clock_ns() - start;
    return pixels;
}
int main(int argc, char **argv) {
    if (argc < 8 || !getenv("GITHUB_ACTIONS")) fail("Actions-only arguments");
    int width = atoi(argv[2]), height = atoi(argv[3]);
    if (width <= 0 || height <= 0 || width > 16384 || height > 16384 || (size_t)width * (size_t)height > SIZE_MAX / 4) fail("frame extent");
    size_t length = (size_t)width * (size_t)height * 4;
    int cold = !strcmp(argv[6], "cold");
    uint64_t start = clock_ns();
    ASS_Library *library = ass_library_init();
    if (!library) fail("ass_library_init");
    unsigned warning_count = 0;
    ass_set_message_cb(library, messages, &warning_count);
    for (int i = 7; i < argc; i++) {
        size_t count; char *font = read_file(argv[i], &count);
        if (count > INT_MAX) fail("font extent");
        ass_add_font(library, argv[i], font, (int)count); free(font);
    }
    uint64_t font_setup = clock_ns() - start;
    start = clock_ns();
    size_t count; char *source = read_file(argv[1], &count);
    ASS_Track *track = ass_read_memory(library, source, count, NULL); free(source);
    if (!track) fail("ass_read_memory");
    uint64_t source_setup = clock_ns() - start;
    FILE *times = fopen(argv[5], "r");
    if (!times) fail("timestamp file");
    ASS_Renderer *renderer = NULL;
    uint64_t initial_setup = 0;
    if (!cold) {
        start = clock_ns(); renderer = new_renderer(library, width, height); initial_setup = clock_ns() - start;
        long long first; if (fscanf(times, "%lld", &first) != 1) fail("warm timestamp"); rewind(times);
        uint64_t r, c; unsigned n; int change;
        free(render_rgba(renderer, track, first, width, height, &r, &c, &n, &change));
    }
    fprintf(stderr, "LIBASS_LIBRARY_VERSION=0x%08x\n", (unsigned)ass_library_version());
    puts("sample,timestamp_ms,total_ns,renderer_setup_ns,font_setup_ns,source_setup_ns,rgba_digest,diagnostic_digest,nonzero_alpha,library_render_ns,client_compose_ns,image_count,changed,warning_count");
    long long milliseconds; unsigned sample = 0;
    while (fscanf(times, "%lld", &milliseconds) == 1) {
        uint64_t setup = sample == 0 ? initial_setup : 0;
        if (cold) {
            if (renderer) ass_renderer_done(renderer);
            start = clock_ns(); renderer = new_renderer(library, width, height); setup = clock_ns() - start;
        }
        uint64_t render, compose; unsigned images; int changed = 0;
        start = clock_ns();
        uint8_t *pixels = render_rgba(renderer, track, milliseconds, width, height, &render, &compose, &images, &changed);
        uint64_t total = clock_ns() - start;
        /* Consume all bytes outside timing; FNV is anti-elision, not a visual oracle. */
        uint64_t checksum = UINT64_C(14695981039346656037), nonzero = 0;
        for (size_t i = 0; i < length; i++) { checksum = (checksum ^ pixels[i]) * UINT64_C(1099511628211); if (i % 4 == 3 && pixels[i]) nonzero++; }
        printf("%u,%lld,%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%016" PRIx64 ",na,%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%u,%d,%u\n", sample++, milliseconds, total, setup, font_setup, source_setup, checksum, nonzero, render, compose, images, changed, warning_count);
        free(pixels);
    }
    fclose(times); if (renderer) ass_renderer_done(renderer); ass_free_track(track); ass_library_done(library);
    return sample ? 0 : 2;
}
