# Stages the CEF runtime next to the built Clay binary.
#
# `cef-dll-sys` downloads and unpacks the Chromium distribution into its build output
# directory, but nothing copies it anywhere useful. `libcef.dll` has to be findable by the
# loader when the process starts, and CEF resolves its resource files (`*.pak`, `icudtl.dat`,
# `locales/`) relative to the executable, so the simplest correct answer is to put everything
# beside the binary rather than in a subdirectory.

param(
    [string]$TargetDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "target\debug")
)

$ErrorActionPreference = "Stop"

function Resolve-CefRuntimeSource {
    param([Parameter(Mandatory = $true)][string]$ResolvedTargetDirectory)

    # Several build hashes can accumulate across rebuilds; the newest libcef.dll wins.
    Get-ChildItem -Path (Join-Path $ResolvedTargetDirectory "build") -Directory -Filter "cef-dll-sys-*" -ErrorAction SilentlyContinue |
        ForEach-Object {
            $runtimePath = Join-Path $_.FullName "out\cef_windows_x86_64"
            $dll = Join-Path $runtimePath "libcef.dll"
            if (Test-Path $dll) {
                [pscustomobject]@{
                    RuntimePath   = $runtimePath
                    LastWriteTime = (Get-Item $dll).LastWriteTime
                }
            }
        } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty RuntimePath
}

$targetDirectory = (Resolve-Path $TargetDirectory).Path
$source = Resolve-CefRuntimeSource -ResolvedTargetDirectory $targetDirectory
if (-not $source) {
    throw "Unable to locate a built CEF runtime under '$targetDirectory\build'. Run 'cargo build -p zed' first."
}

Write-Output "Staging CEF runtime from: $source"

# Only copy when the source is newer, so repeated runs are cheap; libcef.dll alone is ~250 MB.
$copied = 0
foreach ($pattern in @("*.dll", "*.bin", "*.dat", "*.pak", "*.json")) {
    Get-ChildItem -LiteralPath $source -Filter $pattern -File -ErrorAction SilentlyContinue | ForEach-Object {
        $destination = Join-Path $targetDirectory $_.Name
        $existing = Get-Item -LiteralPath $destination -ErrorAction SilentlyContinue
        if (-not $existing -or $existing.LastWriteTime -lt $_.LastWriteTime -or $existing.Length -ne $_.Length) {
            Copy-Item -LiteralPath $_.FullName -Destination $destination -Force
            $copied++
        }
    }
}

$localesSource = Join-Path $source "locales"
if (Test-Path $localesSource) {
    $localesDestination = Join-Path $targetDirectory "locales"
    if (-not (Test-Path $localesDestination)) {
        New-Item -ItemType Directory -Path $localesDestination -Force | Out-Null
    }
    # -Path, not -LiteralPath: the latter would treat the wildcard literally and copy nothing.
    Copy-Item -Path (Join-Path $localesSource "*") -Destination $localesDestination -Recurse -Force
}

Write-Output "Staged $copied file(s) into: $targetDirectory"
if (-not (Test-Path (Join-Path $targetDirectory "libcef.dll"))) {
    throw "libcef.dll is still missing from '$targetDirectory' after staging."
}
Write-Output "libcef.dll is in place."
