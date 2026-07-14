$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$iscc = Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'

if (-not (Test-Path -LiteralPath $iscc)) {
    $iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
}

if (-not (Test-Path -LiteralPath $iscc)) {
    throw 'Inno Setup 6 was not found. Install it before building the setup executable.'
}

Push-Location $projectRoot
try {
    cargo build --release
    Copy-Item 'target\release\process_guard.exe' 'ProcessGuard.exe' -Force
    & $iscc 'installer\ProcessGuard.iss'
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Inno Setup could not write directly to dist (exit code $LASTEXITCODE). Retrying in a temporary output directory."
        $temporaryOutput = Join-Path ([System.IO.Path]::GetTempPath()) ("ProcessGuard-Inno-" + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $temporaryOutput | Out-Null

        & $iscc "/O$temporaryOutput" 'installer\ProcessGuard.iss'
        if ($LASTEXITCODE -ne 0) {
            throw "Inno Setup retry failed with exit code $LASTEXITCODE."
        }

        $temporaryInstaller = Join-Path $temporaryOutput 'ProcessGuard-Setup-1.0.2.exe'
        $distDirectory = Join-Path $projectRoot 'dist'
        New-Item -ItemType Directory -Path $distDirectory -Force | Out-Null
        Copy-Item -LiteralPath $temporaryInstaller -Destination (Join-Path $distDirectory 'ProcessGuard-Setup-1.0.2.exe') -Force
    }
}
finally {
    Pop-Location
}

Write-Host "Installer created: $projectRoot\dist\ProcessGuard-Setup-1.0.2.exe"
