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
 [DllImport("gdi32.dll")] static extern int GetPath(IntPtr dc,[Out] Point[] p,[Out] byte[] t,int n);
 public static object[] Run(byte[] bytes,string family) {
  uint count=0; var resource=AddFontMemResourceEx(bytes,(uint)bytes.Length,IntPtr.Zero,ref count);
  if(resource==IntPtr.Zero || count!=1) throw new Exception("font registration");
  var dc=CreateCompatibleDC(IntPtr.Zero); if(dc==IntPtr.Zero)throw new Exception("DC");
  SetBkMode(dc,1);
  var result=new List<object>();
  try { foreach(uint italic in new uint[]{0,1}) {
   var font=CreateFontW(6400,0,0,0,400,italic,0,0,1,4,0,4,0,family);
   if(font==IntPtr.Zero)throw new Exception("font"); var old=SelectObject(dc,font);
   try {
    var actual=new StringBuilder(256); if(GetTextFaceW(dc,256,actual)==0 || actual.ToString()!=family)throw new Exception("font substitution: "+actual);
    Metrics metrics;if(!GetTextMetricsW(dc,out metrics))throw new Exception("metrics");
    if(!BeginPath(dc)||!TextOutW(dc,0,0,"A",1)||!EndPath(dc))throw new Exception("path");
    int n=GetPath(dc,null,null,0); if(n<=0 || n>4096)throw new Exception("path count");
    var p=new Point[n];var t=new byte[n]; if(GetPath(dc,p,t,n)!=n)throw new Exception("path read");
    var points=new List<int[]>();for(int i=0;i<n;i++)points.Add(new[]{p[i].x,p[i].y,(int)t[i]});
    result.Add(new{family,italic,metrics,points});
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
 if($expected.Count -ne 1 -or (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected[0].sha256) { throw 'Frozen font hash mismatch' }
 $results+=@{sha256=(Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant(); observations=[ItalicProbe]::Run([IO.File]::ReadAllBytes($path),$font[1])}
}
$results | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $env:RUNNER_TEMP 'italic-gdi.json')
Get-Content (Join-Path $env:RUNNER_TEMP 'italic-gdi.json')
