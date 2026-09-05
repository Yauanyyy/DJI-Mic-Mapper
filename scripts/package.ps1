param(
    [switch]$SkipBuild,
    [string]$OutputDirectory = "dist"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseExe = Join-Path $projectRoot "target\release\dji-mic-mapper.exe"
$distDir = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory
} else {
    Join-Path $projectRoot $OutputDirectory
}
$projectRootFull = [IO.Path]::GetFullPath($projectRoot).TrimEnd('\')
$distDirFull = [IO.Path]::GetFullPath($distDir).TrimEnd('\')
$projectChildPrefix = "$projectRootFull\"
if ($distDirFull -eq $projectRootFull -or
    -not $distDirFull.StartsWith($projectChildPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputDirectory must be inside the project directory and cannot be the project root."
}

if (-not $SkipBuild) {
    Push-Location $projectRoot
    try {
        cargo build --locked --release
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release failed"
        }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $releaseExe)) {
    throw "Release executable not found: $releaseExe"
}

if (Test-Path -LiteralPath $distDir) {
    Remove-Item -LiteralPath $distDir -Recurse -Force
}
New-Item -ItemType Directory -Path $distDir -Force | Out-Null
Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $distDir "DJI Mic Mapper.exe") -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "config.toml") -Destination $distDir -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination $distDir -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "LICENSE") -Destination $distDir -Force

Write-Host "Portable package created at: $distDir"
