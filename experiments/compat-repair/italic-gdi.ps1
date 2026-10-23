# CI-only focused GDI observation using pinterf's LOGFONT/TextOut/GetPath route.
# No screenshots or authenticated xy-VSFilter captures are manufactured here.
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class ItalicProbe {
 [StructLayout(LayoutKind.Sequential)] public struct Point { public int x,y; }
 [StructLayout(LayoutKind.Sequential)] public struct Size { public int cx,cy; }
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] public struct Metrics {
  public int height,ascent,descent,internalLeading,externalLeading,aveWidth,maxWidth,weight,overhang,aspectX,aspectY;
  public ushort first,last,defaultChar,breakChar; public byte italic,underlined,struck,pitch,charset;
 }
 [DllImport("gdi32.dll")] static extern IntPtr AddFontMemResourceEx(byte[] b,uint n,IntPtr r,ref uint count);
 [DllImport("gdi32.dll")] static extern bool RemoveFontMemResourceEx(IntPtr f);
 [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
 [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);
 [DllImport("gdi32.dll")] static extern int SetBkMode(IntPtr dc,int mode);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateFontW(int h,int w,int e,int a,int weight,uint italic,uint u,uint s,uint charset,uint output,uint clip,uint quality,uint pitch,string face);
 [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc,IntPtr f);
 [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr f);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern bool GetTextMetricsW(IntPtr dc,out Metrics m);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern int GetTextFaceW(IntPtr dc,int n,StringBuilder face);
 [DllImport("gdi32.dll")] static extern bool BeginPath(IntPtr dc);
 [DllImport("gdi32.dll")] static extern bool EndPath(IntPtr dc);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern bool TextOutW(IntPtr dc,int x,int y,string s,int n);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern bool GetTextExtentPoint32W(IntPtr dc,string s,int n,out Size size);
 [DllImport("gdi32.dll")] static extern int GetPath(IntPtr dc,[Out] Point[] p,[Out] byte[] t,int n);
 public static object[] Run(byte[] bytes,string family) {
  return RunText(bytes,family,"A",true);
 }
 public static object[] RunText(byte[] bytes,string family,string text,bool styles) {
  uint count=0; var resource=AddFontMemResourceEx(bytes,(uint)bytes.Length,IntPtr.Zero,ref count);
  if(resource==IntPtr.Zero || count!=1) throw new Exception("font registration");
  var dc=CreateCompatibleDC(IntPtr.Zero); if(dc==IntPtr.Zero)throw new Exception("DC");
  SetBkMode(dc,1);
  var result=new List<object>();
  try { foreach(int weight in styles ? new int[]{400,700} : new int[]{400}) foreach(uint italic in styles ? new uint[]{0,1} : new uint[]{0}) {
   var font=CreateFontW(6400,0,0,0,weight,italic,0,0,1,4,0,4,0,family);
   if(font==IntPtr.Zero)throw new Exception("font"); var old=SelectObject(dc,font);
   try {
    var actual=new StringBuilder(256); if(GetTextFaceW(dc,256,actual)==0 || actual.ToString()!=family)throw new Exception("font substitution: "+actual);
    Metrics metrics;if(!GetTextMetricsW(dc,out metrics))throw new Exception("metrics");
    Size extent;if(!GetTextExtentPoint32W(dc,text,text.Length,out extent))throw new Exception("extent");
    if(!BeginPath(dc)||!TextOutW(dc,0,0,text,text.Length)||!EndPath(dc))throw new Exception("path");
    int n=GetPath(dc,null,null,0); if(n<=0 || n>4096)throw new Exception("path count");
    var p=new Point[n];var t=new byte[n]; if(GetPath(dc,p,t,n)!=n)throw new Exception("path read");
    var points=new List<int[]>();for(int i=0;i<n;i++)points.Add(new[]{p[i].x,p[i].y,(int)t[i]});
    result.Add(new{family,text,weight,italic,metrics,extent,points});
   }finally{SelectObject(dc,old);DeleteObject(font);}
  }}finally{DeleteDC(dc);RemoveFontMemResourceEx(resource);}
  return result.ToArray();
 }
}
'@
$results=@()
$inventory=Get-Content (Join-Path $PSScriptRoot '../../validation/inventory.json') -Raw | ConvertFrom-Json
foreach($font in @(@('regression/faux-italics/FansubBlock-CFF.otf','FansubBlock-CFF'),@('regression/faux-italics/fansubBlock.ttf','Fansub Block'))) {
 $path=Join-Path $env:TAIYOU_CORPUS $font[0]
 $expected=@($inventory.assets | Where-Object path -eq $font[0])
 $hash=if($expected.Count -eq 1 -and $expected[0].mode -eq '120000'){$expected[0].resolved_sha256}else{$expected[0].sha256}
 if($expected.Count -ne 1 -or (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) { throw 'Frozen font hash mismatch' }
 $results+=@{sha256=(Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant(); observations=[ItalicProbe]::Run([IO.File]::ReadAllBytes($path),$font[1])}
}
$widths=@()
foreach($font in @(@('regression/.fonts/DejaVuSans.ttf','DejaVu Sans'),@('regression/.fonts/Arimo-Regular.ttf','Arimo'))) {
 $path=Join-Path $env:TAIYOU_CORPUS $font[0]
 $expected=@($inventory.assets | Where-Object path -eq $font[0])
 if($expected.Count -ne 1 -or (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected[0].sha256) {throw 'Frozen width-probe font hash mismatch'}
 $rows=foreach($text in @('A','V','AV','T','o','To')) { [ItalicProbe]::RunText([IO.File]::ReadAllBytes($path),$font[1],$text,$false) }
 $widths+=@{sha256=$expected[0].sha256; observations=@($rows)}
}
$results+=@($widths)
$results | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $env:RUNNER_TEMP 'italic-gdi.json')
Get-Content (Join-Path $env:RUNNER_TEMP 'italic-gdi.json')
