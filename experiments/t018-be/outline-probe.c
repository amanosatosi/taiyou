/* CI-only FreeType outline observations, not a rendering implementation. */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include <stdio.h>
#include <stdlib.h>
typedef struct { long points[4096][3]; unsigned used; } Data;
static int add(Data *d,const FT_Vector *p,int type) {
    if(d->used>=4096) return 1;
    d->points[d->used][0]=p->x; d->points[d->used][1]=-p->y; d->points[d->used++][2]=type; return 0;
}
static int move(const FT_Vector *p,void *d) { return add(d,p,6); }
static int line(const FT_Vector *p,void *d) { return add(d,p,2); }
static int quad(const FT_Vector *a,const FT_Vector *b,void *d) { return add(d,a,3)||add(d,b,3); }
static int cubic(const FT_Vector *a,const FT_Vector *b,const FT_Vector *c,void *d) { return add(d,a,4)||add(d,b,4)||add(d,c,4); }
int main(int argc,char **argv) {
    FT_Library lib; FT_Face face; const unsigned chars[]={46,43,182};
    if(argc!=2||FT_Init_FreeType(&lib)||FT_New_Face(lib,argv[1],0,&face)||FT_Set_Char_Size(face,0,5333L*64,72,72)) return 1;
    FT_Outline_Funcs callbacks={move,line,quad,cubic,0,0};
    puts("[");
    for(int mode=0;mode<2;mode++) for(unsigned g=0;g<3;g++) {
        if(FT_Load_Char(face,chars[g],FT_LOAD_NO_BITMAP|(mode==0?FT_LOAD_NO_HINTING:0))) return 2;
        Data data={0}; if(FT_Outline_Decompose(&face->glyph->outline,&callbacks,&data)) return 3;
        printf("%s{\"unhinted\":%s,\"char\":%u,\"native_em\":5333,\"divisor\":64,\"points\":[",mode||g?",":"",mode==0?"true":"false",chars[g]);
        for(unsigned i=0;i<data.used;i++) printf("%s[%ld,%ld,%ld]",i?",":"",data.points[i][0],data.points[i][1],data.points[i][2]);
        puts("]}");
    }
    puts("]"); FT_Done_Face(face); FT_Done_FreeType(lib); return 0;
}
