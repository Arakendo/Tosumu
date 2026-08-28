param(
    [switch]$RefreshReviewed
)

$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$fixturesDirectory = Join-Path $repositoryRoot "docs\overrides\fixtures"
$outputs = @(
    "inspection-header-fixture-v1.tosumu",
    "inspection-populated-fixture-v1.tosumu",
    "inspection-invalid-magic-v1.bin",
    "inspection-truncated-v1.bin",
    "inspection-newer-format-v1.bin"
)

$existing = $outputs | Where-Object {
    Test-Path -LiteralPath (Join-Path $fixturesDirectory $_)
}
if ($existing.Count -gt 0 -and -not $RefreshReviewed) {
    throw "Refusing to replace reviewed fixture(s): $($existing -join ', ')"
}

$freshFixture = Join-Path $fixturesDirectory "inspection-header-fixture-v1.tosumu"
$populatedFixture = Join-Path $fixturesDirectory "inspection-populated-fixture-v1.tosumu"
$temporaryStem = Join-Path $fixturesDirectory ("inspection-refresh-" + [Guid]::NewGuid().ToString("N"))
$temporaryFresh = $temporaryStem + "-fresh.tosumu"
$temporaryPopulated = $temporaryStem + "-populated.tosumu"
$temporaryArtifacts = @(
    $temporaryFresh,
    ($temporaryFresh + ".wal"),
    ($temporaryFresh + ".writer.lock"),
    $temporaryPopulated,
    ($temporaryPopulated + ".wal"),
    ($temporaryPopulated + ".writer.lock")
)

Push-Location $repositoryRoot
try {
    cargo run -p tosumu-cli -- init $temporaryFresh
    cargo run -p tosumu-cli -- init $temporaryPopulated
    cargo run -p tosumu-cli -- put $temporaryPopulated "browser/fixture" "known populated fixture"
}
finally {
    Pop-Location
}

$freshBytes = [System.IO.File]::ReadAllBytes($temporaryFresh)
if ($freshBytes.Length -lt 4096) {
    throw "Reviewed fresh fixture is smaller than page zero."
}
$populatedBytes = [System.IO.File]::ReadAllBytes($temporaryPopulated)

[System.IO.File]::WriteAllBytes($freshFixture, $freshBytes)
[System.IO.File]::WriteAllBytes($populatedFixture, $populatedBytes)

$invalidMagic = [byte[]]$freshBytes.Clone()
$invalidMagic[0] = $invalidMagic[0] -bxor 0xff
[System.IO.File]::WriteAllBytes(
    (Join-Path $fixturesDirectory "inspection-invalid-magic-v1.bin"),
    $invalidMagic
)

[System.IO.File]::WriteAllBytes(
    (Join-Path $fixturesDirectory "inspection-truncated-v1.bin"),
    [byte[]]$freshBytes[0..127]
)

$newerFormat = [byte[]]$freshBytes.Clone()
$currentFormat = [BitConverter]::ToUInt16($freshBytes, 16)
[BitConverter]::GetBytes([UInt16]($currentFormat + 1)).CopyTo($newerFormat, 16)
[System.IO.File]::WriteAllBytes(
    (Join-Path $fixturesDirectory "inspection-newer-format-v1.bin"),
    $newerFormat
)

foreach ($temporaryArtifact in $temporaryArtifacts) {
    if (Test-Path -LiteralPath $temporaryArtifact) {
        Remove-Item -LiteralPath $temporaryArtifact -Force
    }
}

Write-Host "Prepared reviewed inspection fixtures in $fixturesDirectory"
