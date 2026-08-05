param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$target = "wasm32-unknown-unknown"
$artifact = Join-Path $root "target\$target\$Profile\tosumu_inspection_wasm.wasm"
$outDir = Join-Path $root "docs\overrides\js\inspection-wasm"

Push-Location $root
try {
    cargo build -p tosumu-inspection-wasm --target $target --profile $Profile
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    wasm-bindgen $artifact --target web --out-dir $outDir --out-name tosumu_inspection_wasm
}
finally {
    Pop-Location
}
