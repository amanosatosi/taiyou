// Focused Windows reference probe: same LOGFONT/TextOut/GetPath path as pinterf.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
int wmain(int argc, wchar_t **argv) {
    if (argc != 3) return 2;
    HANDLE file=CreateFileW(argv[1],GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
    if(file==INVALID_HANDLE_VALUE) return 3;
    DWORD size=GetFileSize(file,NULL),read=0,count=0;
    void *bytes=malloc(size);
    if(!bytes || !ReadFile(file,bytes,size,&read,NULL) || read!=size) return 4;
    CloseHandle(file);
    HANDLE resource=AddFontMemResourceEx(bytes,size,NULL,&count);
    if(!resource || !count) return 5;
    HDC dc=CreateCompatibleDC(NULL);
    SetBkMode(dc,TRANSPARENT);
    const wchar_t chars[]=L"A\x65e5\x3042\x3001\x3002\x300c\x30fc\xff21";
    for(int vertical=0;vertical<2;vertical++) {
        LOGFONTW lf={0}; lf.lfHeight=1000; lf.lfCharSet=DEFAULT_CHARSET;
        lf.lfOutPrecision=OUT_TT_PRECIS;lf.lfQuality=ANTIALIASED_QUALITY;
        swprintf_s(lf.lfFaceName,LF_FACESIZE,vertical?L"@%ls":L"%ls",argv[2]);
        HFONT font=CreateFontIndirectW(&lf);
        if(!font) return 6;
        HGDIOBJ old=SelectObject(dc,font);
        wchar_t actual[128]; GetTextFaceW(dc,128,actual);
        TEXTMETRICW tm;GetTextMetricsW(dc,&tm);
        printf("FACE vertical=%d actual=",vertical);wprintf(L"%ls",actual);printf(" ascent=%ld descent=%ld\n",tm.tmAscent,tm.tmDescent);
        for(size_t i=0;i<sizeof(chars)/sizeof(chars[0])-1;i++) {
            SIZE extent; WORD glyph;
            if(!GetTextExtentPoint32W(dc,chars+i,1,&extent)) return 7;
            if(GetGlyphIndicesW(dc,chars+i,1,&glyph,0)==GDI_ERROR) return 8;
            BeginPath(dc);TextOutW(dc,0,0,chars+i,1);EndPath(dc);
            int n=GetPath(dc,NULL,NULL,0);
            if(n<0 || n>8192) return 9;
            POINT points[8192];BYTE types[8192];
            if(GetPath(dc,points,types,8192)!=n) return 10;
            LONG x0=LONG_MAX,y0=LONG_MAX,x1=LONG_MIN,y1=LONG_MIN;
            for(int j=0;j<n;j++){if(points[j].x<x0)x0=points[j].x;if(points[j].x>x1)x1=points[j].x;if(points[j].y<y0)y0=points[j].y;if(points[j].y>y1)y1=points[j].y;}
            printf("GLYPH vertical=%d cp=%u glyph=%u advance=%ld,%ld bounds=%ld,%ld,%ld,%ld points=%d first=",vertical,(unsigned)chars[i],(unsigned)glyph,extent.cx,extent.cy,x0,y0,x1,y1,n);
            for(int j=0;j<n && j<12;j++)printf("%ld,%ld/%u;",points[j].x,points[j].y,(unsigned)types[j]);
            printf("\n");
        }
        SelectObject(dc,old);DeleteObject(font);
    }
    DeleteDC(dc);RemoveFontMemResourceEx(resource);free(bytes);return 0;
}
