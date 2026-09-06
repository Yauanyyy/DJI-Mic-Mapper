param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$artifactDir = Join-Path $projectRoot "artifacts"
$portableDir = Join-Path $artifactDir "portable"

if ($Version -notmatch '^\d+\.\d+\.\d+(\.\d+)?$') {
    throw "Version must look like 1.2.3 or 1.2.3.4"
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

if (Test-Path -LiteralPath $artifactDir) {
    Remove-Item -LiteralPath $artifactDir -Recurse -Force
}
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null

& (Join-Path $PSScriptRoot "package.ps1") -SkipBuild -OutputDirectory "artifacts\portable"

$portableArchive = Join-Path $artifactDir "DJI-Mic-Mapper-$Version-windows-x64-portable.zip"
Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $portableArchive -Force

$innoSetup = $null
$command = Get-Command iscc.exe -ErrorAction SilentlyContinue
if ($command) {
    $innoSetup = $command.Source
}

if (-not $innoSetup) {
    $candidates = @(
        (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
        (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe")
    )
    $innoSetup = $candidates |
        Where-Object { $_ -and (Test-Path -LiteralPath $_) } |
        Select-Object -First 1
}

if (-not $innoSetup) {
    throw "Inno Setup 6 was not found. Install it before building an installer."
}

$installerScript = Join-Path $projectRoot "installer\dji-mic-mapper.iss"
& $innoSetup "/DAppVersion=$Version" $installerScript
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed"
}

Write-Host "Release artifacts created in: $artifactDir"
