$ErrorActionPreference = 'Stop'

$version = '1.0.2'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$releaseRoot = Join-Path $projectRoot "GitHub-Release-$version"
$repository = Join-Path $releaseRoot 'repository'
$assets = Join-Path $releaseRoot 'release-assets'

$expectedPrefix = $projectRoot.TrimEnd('\') + '\GitHub-Release-'
if (-not $releaseRoot.StartsWith($expectedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to replace unexpected release path: $releaseRoot"
}

if (Test-Path -LiteralPath $releaseRoot) {
    Remove-Item -LiteralPath $releaseRoot -Recurse -Force
}

New-Item -ItemType Directory -Path $repository,$assets | Out-Null

$rootSources = @(
    '.gitignore',
    'APP_DESCRIPTION.md',
    'Cargo.lock',
    'Cargo.toml',
    'CONTRIBUTING.md',
    'LICENSE',
    'PUBLISHING.md',
    'README.md',
    "RELEASE_NOTES_$version.md",
    'SECURITY.md',
    'build-installer.ps1',
    'build.rs',
    'prepare-release.ps1',
    'ProcessGuard.ico'
)

foreach ($source in $rootSources) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $source) -Destination $repository
}

Copy-Item -LiteralPath (Join-Path $projectRoot 'src') -Destination $repository -Recurse
Copy-Item -LiteralPath (Join-Path $projectRoot 'installer') -Destination $repository -Recurse
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs') -Destination $repository -Recurse

Copy-Item -LiteralPath (Join-Path $projectRoot 'ProcessGuard.exe') -Destination $assets
Copy-Item -LiteralPath (Join-Path $projectRoot "dist\ProcessGuard-Setup-$version.exe") -Destination $assets
Copy-Item -LiteralPath (Join-Path $projectRoot "RELEASE_NOTES_$version.md") -Destination $assets
Copy-Item -LiteralPath (Join-Path $projectRoot 'APP_DESCRIPTION.md') -Destination $releaseRoot
Copy-Item -LiteralPath (Join-Path $projectRoot 'PUBLISHING.md') -Destination $releaseRoot

$sourceZip = Join-Path $assets "ProcessGuard-Source-$version.zip"
Compress-Archive -Path (Join-Path $repository '*') -DestinationPath $sourceZip -CompressionLevel Optimal

$checksumPath = Join-Path $assets 'SHA256SUMS.txt'
$checksumLines = Get-ChildItem -LiteralPath $assets -File |
    Where-Object { $_.Name -ne 'SHA256SUMS.txt' } |
    Sort-Object Name |
    ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash *$($_.Name)"
    }
$checksumLines | Set-Content -LiteralPath $checksumPath -Encoding ascii

Write-Host "GitHub package created: $releaseRoot"
