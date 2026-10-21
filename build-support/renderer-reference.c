/* CI-only independently authored single-H pixel expectation. Uses pinned
 * FreeType font/outline metrics and independent xy eighth-grid coverage,
 * not Taiyou layout/geometry/raster/paint functions. This
 * candidate-side reference is not a trusted T005 oracle or approved golden. */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include FT_TRUETYPE_TABLES_H
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void require(int ok) { if (!ok) { fputs("renderer reference failure\n", stderr); exit(1); } }
static long quantize(double n) { require(isfinite(n) && fabs(n) < 2000000000.0); return (long)round(n); }
typedef struct { int64_t column; int winding; } Crossing;
static int64_t floor_divide(int64_t n, int64_t d) {
    int64_t q=n/d, r=n%d; return q-(r<0);
}
static int crossing_order(const void *a,const void *b) {
    const Crossing *x=a,*y=b;
    if(x->column!=y->column) return x->column<y->column?-1:1;
    return (x->winding>y->winding)-(x->winding<y->winding);
}
/* Independent row sampling, deliberately unlike the production edge heap.
 * Rasterizer.cpp ScanLineData 3207-3358: sample y=8*j+4 in native Q6, use
 * truncated 16.16 slopes, rounded eighth-column crossings and nonzero winding.
 * The fixed H has straight contours; retain a hard check against a changed font.
 * Convert native 0..64 coverage only after accumulation (FillAlphaMashBody_c).
 */
static void eighth_grid(const FT_Outline *outline,unsigned char *data) {
    require(outline->n_points>0 && outline->n_contours>0);
    Crossing *edges=malloc((size_t)outline->n_points*sizeof(*edges));require(edges!=NULL);
    memset(data,0,160*100);
    for(int row=0;row<100*8;row++) {
        int used=0,first=0;int64_t sample=(int64_t)row*8+4;
        for(int contour=0;contour<outline->n_contours;contour++) {
            int last=outline->contours[contour];require(last>=first && last<outline->n_points);
            for(int i=first;i<=last;i++) {
                int next=i==last?first:i+1;
                require(FT_CURVE_TAG(outline->tags[i])==FT_CURVE_TAG_ON);
                int64_t x0=outline->points[i].x, x1=outline->points[next].x;
                int64_t y0=6400-outline->points[i].y, y1=6400-outline->points[next].y;
                if(y0==y1) continue;
                int direction=y0<y1?1:-1;
                if(y0>y1) {int64_t t=x0;x0=x1;x1=t;t=y0;y0=y1;y1=t;}
                int64_t first_row=floor_divide(y0+3,8),last_row=floor_divide(y1-5,8);
                if(row<first_row || row>last_row) continue;
                int64_t slope=((x1-x0)*65536)/(y1-y0);
                int64_t initial=first_row*8+4;
                int64_t crossing=x0*8192+floor_divide(slope*(initial-y0),8)+(row-first_row)*slope;
                require(used<outline->n_points);
                edges[used++]=(Crossing){floor_divide(crossing+32768,65536),direction};
            }first=last+1;
        }
        qsort(edges,(size_t)used,sizeof(*edges),crossing_order);
        int winding=0;int64_t left=0;
        for(int i=0;i<used;i++) {
            if(!winding) left=edges[i].column;
            winding+=edges[i].winding;
            if(!winding) {
                int64_t right=edges[i].column;
                if(left<0)left=0;
                if(right>160*8)right=160*8;
                for(int64_t x=left;x<right;x++) {size_t index=(size_t)(row/8)*160+(size_t)(x/8);require(data[index]<64);data[index]++;}
            }
        }require(winding==0);
    }free(edges);
    for(int i=0;i<160*100;i++) data[i]=(unsigned char)(((unsigned)data[i]*255+32)>>6);
}
static void identical_path_control(void) {
    FT_Vector points[4]={{4,6400},{68,6400},{68,6336},{4,6336}};
    char tags[4]={1,1,1,1};short contours[1]={3};
    FT_Outline outline={1,4,points,tags,contours,0};
    unsigned char bytes[160*100];eighth_grid(&outline,bytes);
    require(bytes[0]==223 && bytes[1]==32);
    for(int i=2;i<160*100;i++) require(bytes[i]==0);
    puts("INDEPENDENT_EIGHTH_CONTROL xy64=56,8 converted=223,32; FT analytical control=240,16");
}
int main(int argc, char **argv) {
    FT_Library library; FT_Face face; int major, minor, patch;
    require(argc == 3 && FT_Init_FreeType(&library) == 0);
    FT_Library_Version(library, &major, &minor, &patch);
    require(major == 2 && minor == 14 && patch == 3);
    identical_path_control();
    require(FT_New_Face(library, argv[1], 0, &face) == 0 && face->units_per_EM > 0);
    TT_OS2 *os2 = FT_Get_Sfnt_Table(face, FT_SFNT_OS2);
    require(os2 != NULL && os2->usWinAscent + os2->usWinDescent > 0);
    double em = floor(1280.0 * face->units_per_EM / (os2->usWinAscent + os2->usWinDescent) + 0.5);
    double scale = em / (64.0 * face->units_per_EM);
    double ascent = floor((floor(os2->usWinAscent * scale * 64.0 + 0.5) + 4.0) / 8.0) / 8.0;
    double descent = floor((floor(os2->usWinDescent * scale * 64.0 + 0.5) + 4.0) / 8.0) / 8.0;
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
        y += floor(os2->usWinAscent * scale * 64.0 + 0.5) / 64.0 - ascent;
        for (unsigned int i = 0; i < outline->n_points; i++) {
            double px = outline->points[i].x * scale + x;
            double py = -outline->points[i].y * scale + y;
            outline->points[i].x = quantize(px * 64.0);
            outline->points[i].y = 100 * 64 - quantize(py * 64.0);
        }
        unsigned char data[160 * 100] = {0};
        eighth_grid(outline,data);
        char path[4096]; int n = snprintf(path, sizeof(path), "%s/align%d.a8", argv[2], a);
        require(n > 0 && (size_t)n < sizeof(path));
        FILE *file = fopen(path, "wb"); require(file != NULL);
        require(fwrite(data, 1, sizeof(data), file) == sizeof(data)); require(fclose(file) == 0);
    }
    require(FT_Done_Face(face) == 0); require(FT_Done_FreeType(library) == 0);
    return 0;
}
