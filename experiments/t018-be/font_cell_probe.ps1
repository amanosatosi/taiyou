# CI-only GDI metric/path observation, not a production backend or xy capture.
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class CellProbe {
  [StructLayout(LayoutKind.Sequential)] public struct Point { public int x,y; }
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] public struct Metrics {
    public int height,ascent,descent,internalLeading,externalLeading,aveWidth,maxWidth,weight,overhang,aspectX,aspectY;
    public ushort first,last,defaultChar,breakChar;
    public byte italic,underlined,struck,pitch,charset;
  }
  [DllImport("gdi32.dll")] static extern IntPtr AddFontMemResourceEx(byte[] bytes,uint size,IntPtr reserved,ref uint count);
  [DllImport("gdi32.dll")] static extern bool RemoveFontMemResourceEx(IntPtr font);
  [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
  [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);
  [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateFontW(int height,int width,int escape,int angle,int weight,uint italic,uint underline,uint strike,uint charset,uint output,uint clip,uint quality,uint pitch,string face);
  [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc,IntPtr font);
  [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr font);
  [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern bool GetTextMetricsW(IntPtr dc,out Metrics metrics);
  [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern int GetTextFaceW(IntPtr dc,int count,StringBuilder name);
  [DllImport("gdi32.dll")] static extern int SetBkMode(IntPtr dc,int mode);
  [DllImport("gdi32.dll")] static extern bool BeginPath(IntPtr dc);
  [DllImport("gdi32.dll")] static extern bool EndPath(IntPtr dc);
  [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] static extern bool TextOutW(IntPtr dc,int x,int y,string text,int count);
  [DllImport("gdi32.dll")] static extern int GetPath(IntPtr dc,Point[] points,byte[] types,int count);
  public static object[] Run(byte[] bytes,string family,int[] heights) {
    uint count=0; var resource=AddFontMemResourceEx(bytes,(uint)bytes.Length,IntPtr.Zero,ref count);
    if(resource==IntPtr.Zero || count!=1) throw new Exception("font resource identity/count");
    var dc=CreateCompatibleDC(IntPtr.Zero); if(dc==IntPtr.Zero) throw new Exception("DC");
    var result=new List<object>(); SetBkMode(dc,1);
    try { foreach(var height in heights) {
      var font=CreateFontW(height,0,0,0,400,0,0,0,1,4,0,4,0,family);
      if(font==IntPtr.Zero) throw new Exception("font"); var old=SelectObject(dc,font);
      try {
        var name=new StringBuilder(256); if(GetTextFaceW(dc,256,name)==0 || name.ToString()!=family) throw new Exception("font substitution: "+name);
        Metrics metrics; if(!GetTextMetricsW(dc,out metrics)) throw new Exception("metrics");
        foreach(var text in new[]{".","+","¶","Ag"}) {
          if(!BeginPath(dc)||!TextOutW(dc,0,0,text,text.Length)||!EndPath(dc)) throw new Exception("path");
          int n=GetPath(dc,null,null,0); if(n<=0||n>10000) throw new Exception("path count");
          var points=new Point[n]; var types=new byte[n]; if(GetPath(dc,points,types,n)!=n) throw new Exception("path read");
          var path=new List<int[]>(); for(int i=0;i<n;i++) path.Add(new[]{points[i].x,points[i].y,(int)types[i]});
          result.Add(new {family,height,text,metrics,path});
        }
      } finally { SelectObject(dc,old); DeleteObject(font); }
    }} finally { DeleteDC(dc); RemoveFontMemResourceEx(resource); }
    return result.ToArray();
  }
}
'@
$lock = Get-Content "$PSScriptRoot/../../build-support/sources.lock.json" -Raw | ConvertFrom-Json
$source = @($lock.sources | Where-Object id -eq 'font')[0]
$ordinary = Join-Path $env:RUNNER_TEMP 'cell-probe-ordinary.otf'
Invoke-WebRequest $source.url -OutFile $ordinary
if ((Get-FileHash $ordinary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $source.sha256) { throw 'ordinary font hash' }
$ailer = Join-Path $env:TAIYOU_CORPUS 'regression/.fonts/Aileron-Regular.otf'
if ((Get-FileHash $ailer -Algorithm SHA256).Hash.ToLowerInvariant() -ne '2762f4fc2ebad8323264aea52ffa2260b86c9677493d3ce2dc4f34e5851d2aa2') { throw 'Aileron hash' }
$results = @()
foreach ($entry in @(@($ordinary,'Source Sans 3'),@($ailer,'Aileron'))) {
  $results += @{ font_sha256=(Get-FileHash $entry[0] -Algorithm SHA256).Hash.ToLowerInvariant(); observations=[CellProbe]::Run([IO.File]::ReadAllBytes($entry[0]),$entry[1],@(1280,1920,6400)) }
}
$output = Join-Path $env:RUNNER_TEMP 't018-gdi-cell-probe.json'
@{ image=$env:ImageVersion; os=[Environment]::OSVersion.VersionString; architecture=$env:PROCESSOR_ARCHITECTURE; candidate=$env:GITHUB_SHA; fonts=$results } | ConvertTo-Json -Depth 20 | Set-Content $output
Get-Content $output
