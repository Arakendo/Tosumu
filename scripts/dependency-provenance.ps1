param(
    [string]$OutputPath = "docs/Notes/dependency-provenance-baseline-v1.json",
    [switch]$Check
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$absoluteOutputPath = if ([IO.Path]::IsPathRooted($OutputPath)) {
    $OutputPath
} else {
    Join-Path $repositoryRoot $OutputPath
}

function Get-LockPackages {
    param([string]$LockPath)

    $packages = [Collections.Generic.List[object]]::new()
    $current = $null

    foreach ($line in [IO.File]::ReadLines($LockPath)) {
        if ($line -eq "[[package]]") {
            if ($null -ne $current) {
                $packages.Add([PSCustomObject]$current)
            }
            $current = [ordered]@{}
            continue
        }

        if ($null -eq $current) {
            continue
        }

        if ($line -match '^name = "(?<value>.*)"$') {
            $current.name = $Matches.value
        } elseif ($line -match '^version = "(?<value>.*)"$') {
            $current.version = $Matches.value
        } elseif ($line -match '^source = "(?<value>.*)"$') {
            $current.source = $Matches.value
        } elseif ($line -match '^checksum = "(?<value>.*)"$') {
            $current.checksum = $Matches.value
        }
    }

    if ($null -ne $current) {
        $packages.Add([PSCustomObject]$current)
    }

    return $packages
}

function Get-LockKey {
    param(
        [string]$Name,
        [string]$Version,
        [AllowNull()][string]$Source
    )

    return "$Name|$Version|$Source"
}

function Invoke-CargoMetadata {
    param([AllowNull()][string]$Target)

    $arguments = @("metadata", "--locked", "--format-version", "1")
    if (-not [string]::IsNullOrEmpty($Target)) {
        $arguments += @("--filter-platform", $Target)
    }

    Push-Location $repositoryRoot
    try {
        $raw = & cargo @arguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo metadata failed for target '$Target' with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    return (($raw -join "`n") | ConvertFrom-Json -Depth 100)
}

function Get-StablePackageId {
    param(
        [object]$Package,
        [string]$Root
    )

    if ($null -ne $Package.source) {
        return "$($Package.source)#$($Package.name)@$($Package.version)"
    }

    $relativeManifest = [IO.Path]::GetRelativePath($Root, [string]$Package.manifest_path)
    $relativeDirectory = Split-Path -Parent $relativeManifest
    $relativeDirectory = $relativeDirectory.Replace('\', '/')
    return "workspace:$relativeDirectory#$($Package.name)@$($Package.version)"
}

function Get-DependencyKind {
    param([AllowNull()]$Kind)

    if ($null -eq $Kind) {
        return "normal"
    }
    return [string]$Kind
}

function Get-NextRole {
    param(
        [string]$CurrentRole,
        [string]$DependencyKind
    )

    switch ($CurrentRole) {
        "workspace" {
            switch ($DependencyKind) {
                "build" { return "build" }
                "dev" { return "development" }
                default { return "normal" }
            }
        }
        "normal" {
            switch ($DependencyKind) {
                "build" { return "build" }
                "dev" { return $null }
                default { return "normal" }
            }
        }
        "build" {
            if ($DependencyKind -eq "dev") {
                return $null
            }
            return "build"
        }
        "development" {
            if ($DependencyKind -eq "dev") {
                return $null
            }
            return "development"
        }
        default {
            throw "Unknown dependency traversal role: $CurrentRole"
        }
    }
}

function Get-Profile {
    param(
        [string]$Name,
        [AllowNull()][string]$Target,
        [object]$Metadata,
        [hashtable]$StableIds
    )

    $nodes = @{}
    foreach ($node in $Metadata.resolve.nodes) {
        $nodes[[string]$node.id] = $node
    }

    $roles = @{}
    $queue = [Collections.Generic.Queue[object]]::new()
    foreach ($memberId in $Metadata.workspace_members) {
        $queue.Enqueue([PSCustomObject]@{ id = [string]$memberId; role = "workspace" })
    }

    while ($queue.Count -gt 0) {
        $item = $queue.Dequeue()
        $roleKey = "$($item.id)|$($item.role)"
        if ($roles.ContainsKey($roleKey)) {
            continue
        }
        $roles[$roleKey] = $true

        $node = $nodes[[string]$item.id]
        foreach ($dependency in $node.deps) {
            foreach ($kind in $dependency.dep_kinds) {
                $dependencyKind = Get-DependencyKind $kind.kind
                $nextRole = Get-NextRole $item.role $dependencyKind
                if ($null -ne $nextRole) {
                    $queue.Enqueue([PSCustomObject]@{
                        id = [string]$dependency.pkg
                        role = $nextRole
                    })
                }
            }
        }
    }

    $profilePackages = foreach ($node in $Metadata.resolve.nodes) {
        $nodeId = [string]$node.id
        $nodeRoles = foreach ($candidate in @("workspace", "normal", "build", "development")) {
            if ($roles.ContainsKey("$nodeId|$candidate")) {
                $candidate
            }
        }
        if (@($nodeRoles).Count -eq 0) {
            continue
        }

        [ordered]@{
            id = $StableIds[$nodeId]
            roles = @($nodeRoles)
            enabled_features = @($node.features | Sort-Object)
        }
    }

    return [ordered]@{
        name = $Name
        target = if ([string]::IsNullOrEmpty($Target)) { $null } else { $Target }
        package_count = @($profilePackages).Count
        packages = @($profilePackages | Sort-Object id)
    }
}

$lockPath = Join-Path $repositoryRoot "Cargo.lock"
$lockPackages = Get-LockPackages $lockPath
$lockIndex = @{}
foreach ($package in $lockPackages) {
    $source = if ($package.PSObject.Properties.Name -contains "source") {
        [string]$package.source
    } else {
        $null
    }
    $lockIndex[(Get-LockKey $package.name $package.version $source)] = $package
}

$profileDefinitions = @(
    [ordered]@{ name = "workspace-unfiltered"; target = $null },
    [ordered]@{ name = "linux-x86_64"; target = "x86_64-unknown-linux-gnu" },
    [ordered]@{ name = "windows-x86_64"; target = "x86_64-pc-windows-msvc" },
    [ordered]@{ name = "macos-x86_64"; target = "x86_64-apple-darwin" },
    [ordered]@{ name = "wasm32-browser"; target = "wasm32-unknown-unknown" }
)

$metadataByProfile = [ordered]@{}
foreach ($definition in $profileDefinitions) {
    $metadataByProfile[$definition.name] = Invoke-CargoMetadata $definition.target
}

$unfiltered = $metadataByProfile["workspace-unfiltered"]
$stableIds = @{}
foreach ($package in $unfiltered.packages) {
    $stableIds[[string]$package.id] = Get-StablePackageId $package $repositoryRoot
}

$catalog = foreach ($package in $unfiltered.packages) {
    $source = if ($null -eq $package.source) { $null } else { [string]$package.source }
    $lockPackage = $lockIndex[(Get-LockKey $package.name $package.version $source)]
    $checksum = if (
        $null -ne $lockPackage -and
        $lockPackage.PSObject.Properties.Name -contains "checksum"
    ) {
        [string]$lockPackage.checksum
    } else {
        $null
    }
    $targetKinds = @($package.targets | ForEach-Object { $_.kind } | ForEach-Object { $_ } | Sort-Object -Unique)

    [ordered]@{
        id = $stableIds[[string]$package.id]
        name = [string]$package.name
        version = [string]$package.version
        source = if ($null -eq $source) { "workspace" } else { $source }
        checksum = $checksum
        checksum_state = if ($null -ne $checksum) { "observed" } elseif ($null -eq $source) { "not_applicable" } else { "unavailable" }
        license = if ($null -eq $package.license) { $null } else { [string]$package.license }
        rust_version = if ($null -eq $package.rust_version) { $null } else { [string]$package.rust_version }
        target_kinds = $targetKinds
        has_build_script = $targetKinds -contains "custom-build"
        is_proc_macro = $targetKinds -contains "proc-macro"
        unsafe_review_state = "not_assessed"
    }
}

$profiles = foreach ($definition in $profileDefinitions) {
    Get-Profile $definition.name $definition.target $metadataByProfile[$definition.name] $stableIds
}

$document = [ordered]@{
    schema = "tosumu-dependency-provenance-baseline"
    schema_version = 1
    subject = [ordered]@{
        kind = "cargo-workspace-lock"
        cargo_lock_sha256 = (Get-FileHash -LiteralPath $lockPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    observation = [ordered]@{
        state = "observed_finding"
        method = "cargo metadata --locked --format-version 1, with explicit target filters"
        limitations = @(
            "Registry checksums are lockfile resolution evidence, not source-audit evidence.",
            "Cargo metadata identifies build-script and procedural-macro targets but does not prove their behavior.",
            "Unsafe-code review is not machine-derived and remains not_assessed for every package.",
            "Target-filtered resolution is dependency-closure evidence, not platform qualification or successful compilation.",
            "Licenses are package metadata observations and have not been independently legally reviewed."
        )
    }
    workspace_members = @($unfiltered.workspace_members | ForEach-Object { $stableIds[[string]$_] } | Sort-Object)
    package_count = @($catalog).Count
    packages = @($catalog | Sort-Object id)
    profiles = @($profiles)
}

$serialized = (($document | ConvertTo-Json -Depth 100) -replace "`r`n", "`n") + "`n"

if ($Check) {
    if (-not (Test-Path -LiteralPath $absoluteOutputPath)) {
        throw "Dependency provenance baseline is missing: $absoluteOutputPath"
    }
    $retained = [IO.File]::ReadAllText($absoluteOutputPath)
    if ($retained -ne $serialized) {
        throw "Dependency provenance baseline is stale. Regenerate with: pwsh -File scripts/dependency-provenance.ps1"
    }
    Write-Host "Dependency provenance baseline matches Cargo.lock and Cargo metadata."
    exit 0
}

$outputDirectory = Split-Path -Parent $absoluteOutputPath
[IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
[IO.File]::WriteAllText($absoluteOutputPath, $serialized, [Text.UTF8Encoding]::new($false))
Write-Host "Wrote dependency provenance baseline to $absoluteOutputPath"
