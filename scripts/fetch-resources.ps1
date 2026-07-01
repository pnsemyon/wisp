<#
    Fetch the pinned sing-box.exe and wintun.dll into resources\ (Windows / PowerShell).
    Usage: .\scripts\fetch-resources.ps1 [-SingboxTag v1.13.14-extended-2.5.0]
    Defaults to the pinned tag below (kept in sync with the config schema
    wisp-core generates). Pass -SingboxTag "" explicitly to instead resolve
    the latest sing-box-extended release via the GitHub API.

    The bundled engine is shtorm-7/sing-box-extended, a fork of mainline
    sing-box (identical config schema) that adds Xray transports, notably
    xhttp, which mainline sing-box doesn't support. See
    crates\wisp-core\src\singbox.rs for the config-generation side of this.
#>
param([string]$SingboxTag = "v1.13.14-extended-2.5.0")

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Res  = Join-Path $Root "resources"
New-Item -ItemType Directory -Force -Path $Res | Out-Null

$WintunVersion = "0.14.1"
$SingboxRepo = "shtorm-7/sing-box-extended"

if ([string]::IsNullOrEmpty($SingboxTag)) {
    Write-Host "Resolving latest sing-box-extended release..."
    # Authenticate the API call when a token is available (e.g. in CI) so we
    # don't hit GitHub's low unauthenticated rate limit on shared runner IPs.
    $headers = @{ "User-Agent" = "wisp-fetch-resources" }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $env:GITHUB_TOKEN" }
    $rel = Invoke-RestMethod "https://api.github.com/repos/$SingboxRepo/releases/latest" -Headers $headers
    $SingboxTag = $rel.tag_name
}
if (-not $SingboxTag.StartsWith("v")) { $SingboxTag = "v$SingboxTag" }
# Asset version is the tag without the leading "v" (e.g. "1.13.14-extended-2.5.0").
$SingboxAssetVersion = $SingboxTag.TrimStart("v")
Write-Host "sing-box-extended tag: $SingboxTag (asset version: $SingboxAssetVersion)"

$Tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP ("wisp-" + [System.Guid]::NewGuid().ToString("N")))
try {
    # --- sing-box (windows amd64), from the xhttp-capable extended fork ---
    # Archive contains a nested sing-box-<asset-version>-windows-amd64\ folder;
    # the recursive Get-ChildItem below handles that regardless of the exact layout.
    $sbZip = "sing-box-$SingboxAssetVersion-windows-amd64.zip"
    $sbUrl = "https://github.com/$SingboxRepo/releases/download/$SingboxTag/$sbZip"
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

    Set-Content -Path (Join-Path $Res ".singbox-version") -Value $SingboxAssetVersion
    Write-Host "Done. Pinned sing-box-extended $SingboxTag, wintun $WintunVersion."
}
finally {
    Remove-Item $Tmp -Recurse -Force -ErrorAction SilentlyContinue
}
