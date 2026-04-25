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

Write-Host "Repository root: $repoRoot"

if (Test-Path $packageCache) {
    Write-Host "Clearing local NuGet cache: $packageCache"
    Remove-Item -Recurse -Force $packageCache
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
    & dotnet test $testProject -c $Configuration -p:RestoreForce=true -p:RestoreNoCache=true
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet test failed with exit code $LASTEXITCODE"
    }
}

Write-Host 'Tosumu .NET package + integration checks completed.'