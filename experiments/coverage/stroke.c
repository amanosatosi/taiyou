/* Independent FreeType public-API experiment, not a production stroke backend. */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include FT_STROKER_H
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static void check(int error) {
    if (error) { fprintf(stderr, "FreeType failure %d\n", error); exit(2); }
}
static void render(FT_Library lib, FT_Outline *outline, const char *dir, const char *id, int radius) {
    FT_BBox box;
    FT_Outline_Get_CBox(outline, &box);
    int x0=(int)floor((double)box.xMin/64), x1=(int)ceil((double)box.xMax/64);
    int y0=(int)floor((double)box.yMin/64), y1=(int)ceil((double)box.yMax/64);
    unsigned w=(unsigned)(x1-x0), h=(unsigned)(y1-y0);
    if (!w || !h || w>4096 || h>4096) exit(3);
    unsigned char *bytes=calloc((size_t)w,h);
    if (!bytes) exit(4);
    FT_Outline_Translate(outline,-x0*64,-y0*64);
    FT_Bitmap bitmap;
    memset(&bitmap,0,sizeof(bitmap));
    bitmap.rows=h; bitmap.width=w; bitmap.pitch=(int)w; bitmap.buffer=bytes;
    bitmap.num_grays=256; bitmap.pixel_mode=FT_PIXEL_MODE_GRAY;
    check(FT_Outline_Get_Bitmap(lib,outline,&bitmap));
    char path[4096];
    if (snprintf(path,sizeof(path),"%s/%s-r%d.a8",dir,id,radius)>=(int)sizeof(path)) exit(5);
    FILE *file=fopen(path,"wb");
    if (!file || fwrite(bytes,1,(size_t)w*h,file)!=(size_t)w*h || fclose(file)) exit(6);
    printf("R002_STROKE case=%s radius_q6=%d bounds=%d,%d,%d,%d mask_bytes=%u ns_samples=",id,radius,x0,y0,x1,y1,w*h);
    for (int sample=0;sample<30;sample++) {
        clock_t start=clock();
        for (int repeat=0;repeat<100;repeat++) {
            memset(bytes,0,(size_t)w*h);
            check(FT_Outline_Get_Bitmap(lib,outline,&bitmap));
        }
        double ns=(double)(clock()-start)*1e9/CLOCKS_PER_SEC/100;
        printf("%s%.0f",sample?",":"",ns);
    }
    printf("\n");
    FT_Outline_Translate(outline,x0*64,y0*64);
    free(bytes);
}
int main(int argc,char **argv) {
    if (argc!=2) return 1;
    FT_Library lib;
    check(FT_Init_FreeType(&lib));
    const FT_Vector rectangle[]={{0,0},{512,0},{512,512},{0,512}};
    const FT_Vector acute[]={{-128,512},{128,0},{144,512},{-128,512}};
    for (int shape=0;shape<2;shape++) {
        FT_Outline source;
        check(FT_Outline_New(lib,4,1,&source));
        memcpy(source.points,shape?acute:rectangle,sizeof(rectangle));
        memset(source.tags,FT_CURVE_TAG_ON,4);
        source.contours[0]=3;
        source.n_points=4; source.n_contours=1;
        check(FT_Outline_Check(&source));
        const int radii[]={0,16,64,1024};
        for (unsigned i=0;i<sizeof(radii)/sizeof(radii[0]);i++) {
            if (radii[i]==0) {
                render(lib,&source,argv[1],shape?"acute":"rectangle",0);
                continue;
            }
            FT_Stroker stroker;
            check(FT_Stroker_New(lib,&stroker));
            FT_Stroker_Set(stroker,radii[i],FT_STROKER_LINECAP_ROUND,FT_STROKER_LINEJOIN_ROUND,0);
            check(FT_Stroker_ParseOutline(stroker,&source,0));
            FT_UInt points,contours;
            FT_StrokerBorder border=FT_Outline_GetOutsideBorder(&source);
            check(FT_Stroker_GetBorderCounts(stroker,border,&points,&contours));
            FT_Outline expanded;
            check(FT_Outline_New(lib,points,contours,&expanded));
            expanded.n_points=0; expanded.n_contours=0;
            FT_Stroker_ExportBorder(stroker,border,&expanded);
            check(FT_Outline_Check(&expanded));
            render(lib,&expanded,argv[1],shape?"acute":"rectangle",radii[i]);
            FT_Outline_Done(lib,&expanded);
            FT_Stroker_Done(stroker);
        }
        FT_Outline_Done(lib,&source);
    }
    FT_Done_FreeType(lib);
    return 0;
}
