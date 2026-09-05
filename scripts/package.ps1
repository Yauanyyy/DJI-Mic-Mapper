param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseExe = Join-Path $projectRoot "target\release\dji-mic-mapper.exe"
$distDir = Join-Path $projectRoot "dist"

if (-not $SkipBuild) {
    Push-Location $projectRoot
    try {
        cargo build --release
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $releaseExe)) {
    throw "Release executable not found: $releaseExe"
}

New-Item -ItemType Directory -Path $distDir -Force | Out-Null
Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $distDir "DJI Mic Mapper.exe") -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "config.toml") -Destination $distDir -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination $distDir -Force

Write-Host "Portable package created at: $distDir"
