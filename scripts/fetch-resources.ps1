<#
    Fetch the pinned sing-box.exe and wintun.dll into resources\ (Windows / PowerShell).
    Usage: .\scripts\fetch-resources.ps1 [-SingboxVersion 1.11.0]
    If no version is given, the latest sing-box release is used.
#>
param([string]$SingboxVersion = "")

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Res  = Join-Path $Root "resources"
New-Item -ItemType Directory -Force -Path $Res | Out-Null

$WintunVersion = "0.14.1"

if ([string]::IsNullOrEmpty($SingboxVersion)) {
    Write-Host "Resolving latest sing-box release..."
    $rel = Invoke-RestMethod "https://api.github.com/repos/SagerNet/sing-box/releases/latest"
    $SingboxVersion = $rel.tag_name.TrimStart("v")
}
$SingboxVersion = $SingboxVersion.TrimStart("v")
Write-Host "sing-box version: $SingboxVersion"

$Tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP ("wisp-" + [System.Guid]::NewGuid().ToString("N")))
try {
    # --- sing-box (windows amd64) ---
    $sbZip = "sing-box-$SingboxVersion-windows-amd64.zip"
    $sbUrl = "https://github.com/SagerNet/sing-box/releases/download/v$SingboxVersion/$sbZip"
    Write-Host "Downloading $sbUrl"
    Invoke-WebRequest $sbUrl -OutFile (Join-Path $Tmp $sbZip)
    Expand-Archive (Join-Path $Tmp $sbZip) -DestinationPath (Join-Path $Tmp "sb") -Force
    $sbExe = Get-ChildItem -Path (Join-Path $Tmp "sb") -Recurse -Filter "sing-box.exe" | Select-Object -First 1
    Copy-Item $sbExe.FullName (Join-Path $Res "sing-box.exe") -Force
    Write-Host "-> resources\sing-box.exe"

    # --- wintun.dll (amd64) ---
    $wtZip = "wintun-$WintunVersion.zip"
    $wtUrl = "https://www.wintun.net/builds/$wtZip"
    Write-Host "Downloading $wtUrl"
    Invoke-WebRequest $wtUrl -OutFile (Join-Path $Tmp $wtZip)
    Expand-Archive (Join-Path $Tmp $wtZip) -DestinationPath (Join-Path $Tmp "wt") -Force
    Copy-Item (Join-Path $Tmp "wt\wintun\bin\amd64\wintun.dll") (Join-Path $Res "wintun.dll") -Force
    Write-Host "-> resources\wintun.dll"

    Set-Content -Path (Join-Path $Res ".singbox-version") -Value $SingboxVersion
    Write-Host "Done. Pinned sing-box v$SingboxVersion, wintun $WintunVersion."
}
finally {
    Remove-Item $Tmp -Recurse -Force -ErrorAction SilentlyContinue
}
