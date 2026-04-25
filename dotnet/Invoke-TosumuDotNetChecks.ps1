[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [switch]$SkipPack,
    [switch]$SkipTests
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$packageProject = Join-Path $repoRoot 'dotnet\Tosumu.Cli\Tosumu.Cli.csproj'
$testProject = Join-Path $repoRoot 'dotnet\Tosumu.Cli.IntegrationTests\Tosumu.Cli.IntegrationTests.csproj'
$packageOutput = Join-Path $repoRoot 'dotnet\_packages'
$packageCache = Join-Path $repoRoot 'dotnet\_packages-cache'
$restorePackagesPath = $packageCache

function Remove-CacheDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Label
    )

    if (-not (Test-Path $Path)) {
        return $true
    }

    try {
        Remove-Item -Recurse -Force $Path
        return $true
    }
    catch {
        Write-Warning ("Could not remove {0}: {1}" -f $Label, $Path)
        return $false
    }
}

function Remove-StaleRestoreCaches {
    param(
        [Parameter(Mandatory = $true)]
        [string]$DotnetRoot,
        [string]$ActivePath
    )

    Get-ChildItem $DotnetRoot -Directory -Filter '_packages-cache-*' -ErrorAction SilentlyContinue |
        Where-Object { -not $ActivePath -or $_.FullName -ne $ActivePath } |
        ForEach-Object {
            Write-Host "Removing stale fallback cache: $($_.FullName)"
            [void](Remove-CacheDirectory -Path $_.FullName -Label 'stale fallback cache')
        }
}

Write-Host "Repository root: $repoRoot"
Remove-StaleRestoreCaches -DotnetRoot (Join-Path $repoRoot 'dotnet')

if (Test-Path $packageCache) {
    Write-Host "Clearing local NuGet cache: $packageCache"
    if (-not (Remove-CacheDirectory -Path $packageCache -Label 'local NuGet cache')) {
        $timestamp = Get-Date -Format 'yyyyMMddHHmmssfff'
        $restorePackagesPath = Join-Path $repoRoot ("dotnet\_packages-cache-$timestamp")
        Write-Warning "Could not clear local NuGet cache. Falling back to fresh restore path: $restorePackagesPath"
    }
}

if (-not $SkipPack) {
    Write-Host "Packing Tosumu.Cli to $packageOutput"
    & dotnet pack $packageProject -c $Configuration -o $packageOutput
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet pack failed with exit code $LASTEXITCODE"
    }
}

if (-not $SkipTests) {
    Write-Host "Running Tosumu.Cli integration tests"
    & dotnet test $testProject -c $Configuration -p:RestoreForce=true -p:RestoreNoCache=true -p:RestorePackagesPath=$restorePackagesPath
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet test failed with exit code $LASTEXITCODE"
    }
}

if ($restorePackagesPath -ne $packageCache) {
    Write-Host "Cleaning fallback restore path: $restorePackagesPath"
    [void](Remove-CacheDirectory -Path $restorePackagesPath -Label 'fallback restore path')
}

Write-Host 'Tosumu .NET package + integration checks completed.'