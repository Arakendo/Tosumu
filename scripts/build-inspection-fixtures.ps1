param()

$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$fixturesDirectory = Join-Path $repositoryRoot "docs\overrides\fixtures"
$outputs = @(
    "inspection-populated-fixture-v1.tosumu",
    "inspection-invalid-magic-v1.bin",
    "inspection-truncated-v1.bin",
    "inspection-newer-format-v1.bin"
)

$existing = $outputs | Where-Object {
    Test-Path -LiteralPath (Join-Path $fixturesDirectory $_)
}
if ($existing.Count -gt 0) {
    throw "Refusing to replace reviewed fixture(s): $($existing -join ', ')"
}

$freshFixture = Join-Path $fixturesDirectory "inspection-header-fixture-v1.tosumu"
if (-not (Test-Path -LiteralPath $freshFixture)) {
    throw "Missing reviewed fresh fixture: $freshFixture"
}

$populatedFixture = Join-Path $fixturesDirectory "inspection-populated-fixture-v1.tosumu"
Push-Location $repositoryRoot
try {
    cargo run -p tosumu-cli -- init $populatedFixture
    cargo run -p tosumu-cli -- put $populatedFixture "browser/fixture" "known populated fixture"
}
finally {
    Pop-Location
}

$freshBytes = [System.IO.File]::ReadAllBytes($freshFixture)
if ($freshBytes.Length -lt 4096) {
    throw "Reviewed fresh fixture is smaller than page zero."
}

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
[BitConverter]::GetBytes([UInt16]3).CopyTo($newerFormat, 16)
[System.IO.File]::WriteAllBytes(
    (Join-Path $fixturesDirectory "inspection-newer-format-v1.bin"),
    $newerFormat
)

Write-Host "Prepared reviewed inspection fixtures in $fixturesDirectory"
