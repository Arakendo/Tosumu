param(
    [string]$OutputPath = "docs/Notes/dependency-provenance-baseline-v1.json",
    [string]$RiskPath = "docs/Notes/dependency-risk-classification-v1.json",
    [string]$BuildReviewPath = "docs/Notes/dependency-build-script-review-v1.json",
    [string]$ExecutableReviewPath = "docs/Notes/dependency-executable-input-review-v1.json",
    [string]$ProcMacroReviewPath = "docs/Notes/dependency-proc-macro-runtime-review-v1.json",
    [switch]$Check
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$requiredRustRelease = "1.95.0"
$rustVersion = (& rustc --version).Trim()
if ($LASTEXITCODE -ne 0 -or $rustVersion -notmatch "^rustc $([regex]::Escape($requiredRustRelease)) ") {
    throw "Dependency provenance requires rustc $requiredRustRelease; observed '$rustVersion'. Run the generator under the admitted toolchain."
}
$cargoVersion = (& cargo --version).Trim()
if ($LASTEXITCODE -ne 0 -or $cargoVersion -notmatch "^cargo $([regex]::Escape($requiredRustRelease)) ") {
    throw "Dependency provenance requires Cargo $requiredRustRelease; observed '$cargoVersion'. Run the generator under the admitted toolchain."
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$absoluteOutputPath = if ([IO.Path]::IsPathRooted($OutputPath)) {
    $OutputPath
} else {
    Join-Path $repositoryRoot $OutputPath
}
$absoluteRiskPath = if ([IO.Path]::IsPathRooted($RiskPath)) {
    $RiskPath
} else {
    Join-Path $repositoryRoot $RiskPath
}
$absoluteBuildReviewPath = if ([IO.Path]::IsPathRooted($BuildReviewPath)) {
    $BuildReviewPath
} else {
    Join-Path $repositoryRoot $BuildReviewPath
}
$absoluteExecutableReviewPath = if ([IO.Path]::IsPathRooted($ExecutableReviewPath)) {
    $ExecutableReviewPath
} else {
    Join-Path $repositoryRoot $ExecutableReviewPath
}
$absoluteProcMacroReviewPath = if ([IO.Path]::IsPathRooted($ProcMacroReviewPath)) {
    $ProcMacroReviewPath
} else {
    Join-Path $repositoryRoot $ProcMacroReviewPath
}

function Get-SourceTreeHash {
    param(
        [string]$PackageRoot,
        [object[]]$Files
    )

    $identity = @(
        $Files |
            Sort-Object FullName |
            ForEach-Object {
                $relative = [IO.Path]::GetRelativePath($PackageRoot, $_.FullName).Replace('\', '/')
                $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
                "$relative`0$hash`n"
            }
    ) -join ""
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($identity)
    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        return ([Convert]::ToHexString($sha256.ComputeHash($bytes))).ToLowerInvariant()
    } finally {
        $sha256.Dispose()
    }
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

function Get-CoreArtifactProfile {
    param(
        [string]$Name,
        [string]$Target,
        [object[]]$Catalog
    )

    Push-Location $repositoryRoot
    try {
        $lines = & cargo tree -p tosumu-core --locked --target $Target `
            --edges normal,build --prefix depth --format '{p}|{f}' --color never
        if ($LASTEXITCODE -ne 0) {
            throw "cargo tree failed for target '$Target' with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $catalogByCoordinate = @{}
    $catalogByStableId = @{}
    foreach ($package in $Catalog) {
        $coordinate = "$($package.name)@$($package.version)"
        if ($catalogByCoordinate.ContainsKey($coordinate)) {
            throw "Ambiguous package coordinate in Cargo closure: $coordinate"
        }
        $catalogByCoordinate[$coordinate] = $package
        $catalogByStableId[[string]$package.id] = $package
    }

    $observed = @{}
    foreach ($line in $lines) {
        if ($line -notmatch '^\d+(?<name>\S+) v(?<version>[^ |]+).*\|(?<features>.*)$') {
            throw "Unexpected cargo tree output for target '$Target': $line"
        }
        $coordinate = "$($Matches.name)@$($Matches.version)"
        if (-not $catalogByCoordinate.ContainsKey($coordinate)) {
            throw "cargo tree package is absent from Cargo metadata: $coordinate"
        }
        $package = $catalogByCoordinate[$coordinate]
        $packageId = [string]$package.id
        if (-not $observed.ContainsKey($packageId)) {
            $observed[$packageId] = @{}
        }
        $featureText = $Matches.features -replace ' \(\*\)$', ''
        foreach ($feature in ($featureText -split ',')) {
            if (-not [string]::IsNullOrWhiteSpace($feature)) {
                $observed[$packageId][$feature.Trim()] = $true
            }
        }
    }

    $packages = foreach ($entry in $observed.GetEnumerator()) {
        [ordered]@{
            id = [string]$entry.Key
            enabled_features = @($entry.Value.Keys | Sort-Object)
        }
    }
    $packageRecords = @($observed.Keys | ForEach-Object { $catalogByStableId[[string]$_] })

    return [ordered]@{
        name = $Name
        package = "tosumu-core"
        target = $Target
        method = "cargo tree -p tosumu-core --locked --target <target> --edges normal,build"
        participation_claim = "reachable_for_selected_package_target_and_features"
        package_count = @($packages).Count
        packages = @($packages | Sort-Object { $_.id })
        build_script_candidates = @(
            $packageRecords |
                Where-Object has_build_script |
                ForEach-Object { [string]$_.id } |
                Sort-Object
        )
        procedural_macro_candidates = @(
            $packageRecords |
                Where-Object is_proc_macro |
                ForEach-Object { [string]$_.id } |
                Sort-Object
        )
    }
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
        packages = @($profilePackages | Sort-Object { $_.id })
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
    [ordered]@{ name = "ios-device-arm64"; target = "aarch64-apple-ios" },
    [ordered]@{ name = "ios-simulator-arm64"; target = "aarch64-apple-ios-sim" },
    [ordered]@{ name = "android-device-arm64"; target = "aarch64-linux-android" },
    [ordered]@{ name = "android-emulator-x86_64"; target = "x86_64-linux-android" },
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

$riskDocument = Get-Content -LiteralPath $absoluteRiskPath -Raw | ConvertFrom-Json -Depth 100
if ($riskDocument.schema -ne "tosumu-dependency-risk-classification" -or $riskDocument.schema_version -ne 1) {
    throw "Unsupported dependency risk classification schema: $absoluteRiskPath"
}

$catalogIds = @{}
foreach ($package in $catalog) {
    $catalogIds[[string]$package.id] = $true
}
$tierRanks = @{ standard = 1; elevated = 2; critical = 3 }
$seenRiskIds = @{}
foreach ($classification in $riskDocument.classifications) {
    $classificationId = [string]$classification.id
    if (-not $catalogIds.ContainsKey($classificationId)) {
        throw "Risk classification does not match the resolved closure: $classificationId"
    }
    if ($seenRiskIds.ContainsKey($classificationId)) {
        throw "Duplicate risk classification: $classificationId"
    }
    $seenRiskIds[$classificationId] = $true

    $tier = [string]$classification.tier
    $tierFloor = [string]$classification.tier_floor
    if (-not $tierRanks.ContainsKey($tier) -or -not $tierRanks.ContainsKey($tierFloor)) {
        throw "Unknown risk tier for $classificationId"
    }
    if ($tierRanks[$tier] -lt $tierRanks[$tierFloor]) {
        throw "Risk tier for $classificationId is below its retained floor"
    }
    if ([string]::IsNullOrWhiteSpace([string]$classification.rationale)) {
        throw "Risk classification lacks rationale: $classificationId"
    }
    if ([string]::IsNullOrWhiteSpace([string]$classification.update_owner)) {
        throw "Risk classification lacks an update owner: $classificationId"
    }
    if (@($classification.concerns).Count -eq 0) {
        throw "Risk classification lacks concerns: $classificationId"
    }
}

$corePackage = $unfiltered.packages | Where-Object { $_.name -eq "tosumu-core" }
if (@($corePackage).Count -ne 1) {
    throw "Expected exactly one tosumu-core workspace package"
}
$coreNode = $unfiltered.resolve.nodes | Where-Object { $_.id -eq $corePackage.id }
$directCoreNormalIds = @(
    $coreNode.deps |
        Where-Object {
            @($_.dep_kinds | Where-Object { (Get-DependencyKind $_.kind) -eq "normal" }).Count -gt 0
        } |
        ForEach-Object { $stableIds[[string]$_.pkg] } |
        Sort-Object -Unique
)
foreach ($directId in $directCoreNormalIds) {
    if (-not $seenRiskIds.ContainsKey($directId)) {
        throw "Direct tosumu-core normal dependency lacks a risk classification: $directId"
    }
}

$normalizedRiskClassifications = foreach ($classification in $riskDocument.classifications) {
    [ordered]@{
        id = [string]$classification.id
        tier = [string]$classification.tier
        tier_floor = [string]$classification.tier_floor
        concerns = @($classification.concerns | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        update_owner = [string]$classification.update_owner
        rationale = [string]$classification.rationale
    }
}

$stableToMetadataId = @{}
foreach ($entry in $stableIds.GetEnumerator()) {
    $stableToMetadataId[[string]$entry.Value] = [string]$entry.Key
}
$classificationById = @{}
foreach ($classification in $normalizedRiskClassifications) {
    $classificationById[[string]$classification.id] = $classification
}
$unfilteredNodes = @{}
foreach ($node in $unfiltered.resolve.nodes) {
    $unfilteredNodes[[string]$node.id] = $node
}
$riskExposure = @{}
foreach ($directId in $directCoreNormalIds) {
    $rootClassification = $classificationById[$directId]
    $queue = [Collections.Generic.Queue[object]]::new()
    $queue.Enqueue([PSCustomObject]@{
        id = $stableToMetadataId[$directId]
        role = "normal"
    })
    $visited = @{}

    while ($queue.Count -gt 0) {
        $item = $queue.Dequeue()
        $visitKey = "$($item.id)|$($item.role)"
        if ($visited.ContainsKey($visitKey)) {
            continue
        }
        $visited[$visitKey] = $true

        $stableId = $stableIds[[string]$item.id]
        if (-not $riskExposure.ContainsKey($stableId)) {
            $riskExposure[$stableId] = [ordered]@{
                roots = @{}
                roles = @{}
                inherited_floor = "standard"
            }
        }
        $riskExposure[$stableId].roots[$directId] = $true
        $riskExposure[$stableId].roles[[string]$item.role] = $true
        if ($tierRanks[[string]$rootClassification.tier_floor] -gt $tierRanks[$riskExposure[$stableId].inherited_floor]) {
            $riskExposure[$stableId].inherited_floor = [string]$rootClassification.tier_floor
        }

        $node = $unfilteredNodes[[string]$item.id]
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
}

$catalogById = @{}
foreach ($package in $catalog) {
    $catalogById[[string]$package.id] = $package
}
$normalizedRiskExposure = foreach ($entry in $riskExposure.GetEnumerator()) {
    $package = $catalogById[[string]$entry.Key]
    [ordered]@{
        id = [string]$entry.Key
        inherited_floor = [string]$entry.Value.inherited_floor
        roles = @($entry.Value.roles.Keys | Sort-Object)
        direct_roots = @($entry.Value.roots.Keys | Sort-Object)
        has_build_script = [bool]$package.has_build_script
        is_proc_macro = [bool]$package.is_proc_macro
        review_state = "not_assessed"
    }
}

$profiles = foreach ($definition in $profileDefinitions) {
    Get-Profile $definition.name $definition.target $metadataByProfile[$definition.name] $stableIds
}
$coreArtifactProfiles = foreach ($definition in ($profileDefinitions | Where-Object { $null -ne $_.target })) {
    Get-CoreArtifactProfile "tosumu-core-$($definition.name)" $definition.target $catalog
}

$buildReviewDocument = Get-Content -LiteralPath $absoluteBuildReviewPath -Raw | ConvertFrom-Json -Depth 100
if ($buildReviewDocument.schema -ne "tosumu-dependency-build-script-review" -or $buildReviewDocument.schema_version -ne 1) {
    throw "Unsupported dependency build-script review schema: $absoluteBuildReviewPath"
}
$requiredBuildReviewIds = @(
    $coreArtifactProfiles.build_script_candidates | Sort-Object -Unique
)
$metadataByStableId = @{}
foreach ($package in $unfiltered.packages) {
    $metadataByStableId[$stableIds[[string]$package.id]] = $package
}
$seenBuildReviewIds = @{}
$normalizedBuildReviews = foreach ($review in $buildReviewDocument.reviews) {
    $reviewId = [string]$review.id
    if ($reviewId -notin $requiredBuildReviewIds) {
        throw "Build-script review is not a current core target candidate: $reviewId"
    }
    if ($seenBuildReviewIds.ContainsKey($reviewId)) {
        throw "Duplicate build-script review: $reviewId"
    }
    $seenBuildReviewIds[$reviewId] = $true
    $package = $metadataByStableId[$reviewId]
    $buildTarget = $package.targets | Where-Object { $_.kind -contains "custom-build" }
    if (@($buildTarget).Count -ne 1) {
        throw "Expected exactly one build-script target for $reviewId"
    }
    $observedHash = (Get-FileHash -LiteralPath $buildTarget.src_path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($observedHash -ne [string]$review.build_script_sha256) {
        throw "Build-script source identity changed for $reviewId"
    }
    if ([string]::IsNullOrWhiteSpace([string]$review.finding) -or @($review.capabilities).Count -eq 0) {
        throw "Build-script review is incomplete for $reviewId"
    }
    [ordered]@{
        id = $reviewId
        build_script_sha256 = $observedHash
        targets = @($review.targets | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        capabilities = @($review.capabilities | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        finding = [string]$review.finding
    }
}
foreach ($requiredId in $requiredBuildReviewIds) {
    if (-not $seenBuildReviewIds.ContainsKey($requiredId)) {
        throw "Core target build-script candidate lacks source review: $requiredId"
    }
}

$executableReviewDocument = Get-Content -LiteralPath $absoluteExecutableReviewPath -Raw | ConvertFrom-Json -Depth 100
if ($executableReviewDocument.schema -ne "tosumu-dependency-executable-input-review" -or $executableReviewDocument.schema_version -ne 1) {
    throw "Unsupported dependency executable-input review schema: $absoluteExecutableReviewPath"
}
$requiredExecutableReviewIds = @(
    "registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.106",
    "registry+https://github.com/rust-lang/crates.io-index#thiserror-impl@2.0.18",
    "registry+https://github.com/rust-lang/crates.io-index#thiserror@2.0.18",
    "registry+https://github.com/rust-lang/crates.io-index#version_check@0.9.5"
)
$seenExecutableReviewIds = @{}
$normalizedExecutableReviews = foreach ($subject in $executableReviewDocument.subjects) {
    $subjectId = [string]$subject.id
    if ($subjectId -notin $requiredExecutableReviewIds -or -not $metadataByStableId.ContainsKey($subjectId)) {
        throw "Executable-input review is not a required current subject: $subjectId"
    }
    if ($seenExecutableReviewIds.ContainsKey($subjectId)) {
        throw "Duplicate executable-input review: $subjectId"
    }
    $seenExecutableReviewIds[$subjectId] = $true
    $package = $metadataByStableId[$subjectId]
    $packageRoot = Split-Path -Parent ([string]$package.manifest_path)
    $selectedPath = [IO.Path]::GetFullPath((Join-Path $packageRoot ([string]$subject.relative_path)))
    $rootPrefix = [IO.Path]::GetFullPath($packageRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $selectedPath.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Executable-input selection escapes package root: $subjectId"
    }
    $files = @(switch ([string]$subject.selection) {
        "file" { @(Get-Item -LiteralPath $selectedPath) }
        "rust_source_tree" { @(Get-ChildItem -LiteralPath $selectedPath -Recurse -File -Filter "*.rs") }
        default { throw "Unknown executable-input selection for $subjectId" }
    })
    if ($files.Count -ne [int]$subject.file_count) {
        throw "Executable-input file count changed for $subjectId"
    }
    $observedHash = Get-SourceTreeHash $packageRoot $files
    if ($observedHash -ne [string]$subject.source_tree_sha256) {
        throw "Executable-input source identity changed for $subjectId"
    }
    if ([string]::IsNullOrWhiteSpace([string]$subject.finding) -or [string]::IsNullOrWhiteSpace([string]$subject.limitations)) {
        throw "Executable-input review lacks finding or limitations: $subjectId"
    }
    [ordered]@{
        id = $subjectId
        selection = [string]$subject.selection
        relative_path = [string]$subject.relative_path
        file_count = $files.Count
        source_tree_sha256 = $observedHash
        capabilities = @($subject.capabilities | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        finding = [string]$subject.finding
        limitations = [string]$subject.limitations
    }
}
foreach ($requiredId in $requiredExecutableReviewIds) {
    if (-not $seenExecutableReviewIds.ContainsKey($requiredId)) {
        throw "Required executable-input subject lacks source review: $requiredId"
    }
}

$procMacroReviewDocument = Get-Content -LiteralPath $absoluteProcMacroReviewPath -Raw | ConvertFrom-Json -Depth 100
if ($procMacroReviewDocument.schema -ne "tosumu-dependency-proc-macro-runtime-review" -or $procMacroReviewDocument.schema_version -ne 1) {
    throw "Unsupported proc-macro runtime review schema: $absoluteProcMacroReviewPath"
}
$requiredProcMacroRuntimeIds = @(
    "registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.106",
    "registry+https://github.com/rust-lang/crates.io-index#quote@1.0.45",
    "registry+https://github.com/rust-lang/crates.io-index#syn@2.0.117",
    "registry+https://github.com/rust-lang/crates.io-index#unicode-ident@1.0.24"
)
$coreFeaturesById = @{}
foreach ($profilePackage in $coreArtifactProfiles.packages) {
    $profileId = [string]$profilePackage.id
    if (-not $coreFeaturesById.ContainsKey($profileId)) {
        $coreFeaturesById[$profileId] = @{}
    }
    foreach ($feature in $profilePackage.enabled_features) {
        $coreFeaturesById[$profileId][[string]$feature] = $true
    }
}
$seenProcMacroRuntimeIds = @{}
$normalizedProcMacroReviews = foreach ($review in $procMacroReviewDocument.reviews) {
    $reviewId = [string]$review.id
    if ($reviewId -notin $requiredProcMacroRuntimeIds -or -not $metadataByStableId.ContainsKey($reviewId)) {
        throw "Proc-macro runtime review is not a required current subject: $reviewId"
    }
    if ($seenProcMacroRuntimeIds.ContainsKey($reviewId)) {
        throw "Duplicate proc-macro runtime review: $reviewId"
    }
    $seenProcMacroRuntimeIds[$reviewId] = $true
    $package = $metadataByStableId[$reviewId]
    $packageRoot = Split-Path -Parent ([string]$package.manifest_path)
    $sourceRoot = [IO.Path]::GetFullPath((Join-Path $packageRoot ([string]$review.relative_path)))
    $files = @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Filter "*.rs")
    if ($files.Count -ne [int]$review.file_count) {
        throw "Proc-macro runtime source file count changed for $reviewId"
    }
    $lineCount = 0
    foreach ($file in $files) {
        $lineCount += @(Get-Content -LiteralPath $file.FullName).Count
    }
    if ($lineCount -ne [int]$review.source_line_count) {
        throw "Proc-macro runtime source line count changed for $reviewId"
    }
    $observedHash = Get-SourceTreeHash $packageRoot $files
    if ($observedHash -ne [string]$review.source_tree_sha256) {
        throw "Proc-macro runtime source identity changed for $reviewId"
    }
    $observedFeatures = @($coreFeaturesById[$reviewId].Keys | Sort-Object)
    $reviewedFeatures = @($review.selected_features | ForEach-Object { [string]$_ } | Sort-Object -Unique)
    if (($observedFeatures -join "`n") -ne ($reviewedFeatures -join "`n")) {
        $observedDisplay = $observedFeatures -join ", "
        $reviewedDisplay = $reviewedFeatures -join ", "
        throw "Proc-macro runtime selected features changed for ${reviewId}: observed [$observedDisplay], reviewed [$reviewedDisplay]"
    }
    if ([string]::IsNullOrWhiteSpace([string]$review.finding) -or [string]::IsNullOrWhiteSpace([string]$review.limitations)) {
        throw "Proc-macro runtime review lacks finding or limitations: $reviewId"
    }
    [ordered]@{
        id = $reviewId
        relative_path = [string]$review.relative_path
        file_count = $files.Count
        source_line_count = $lineCount
        source_tree_sha256 = $observedHash
        selected_features = $observedFeatures
        observed_capabilities = @($review.observed_capabilities | ForEach-Object { [string]$_ } | Sort-Object -Unique)
        finding = [string]$review.finding
        limitations = [string]$review.limitations
    }
}
foreach ($requiredId in $requiredProcMacroRuntimeIds) {
    if (-not $seenProcMacroRuntimeIds.ContainsKey($requiredId)) {
        throw "Required proc-macro runtime subject lacks review: $requiredId"
    }
}

$document = [ordered]@{
    schema = "tosumu-dependency-provenance-baseline"
    schema_version = 1
    subject = [ordered]@{
        kind = "cargo-workspace-lock"
        rustc = $rustVersion
        cargo = $cargoVersion
        cargo_lock_sha256 = (Get-FileHash -LiteralPath $lockPath -Algorithm SHA256).Hash.ToLowerInvariant()
        risk_classification_sha256 = (Get-FileHash -LiteralPath $absoluteRiskPath -Algorithm SHA256).Hash.ToLowerInvariant()
        build_script_review_sha256 = (Get-FileHash -LiteralPath $absoluteBuildReviewPath -Algorithm SHA256).Hash.ToLowerInvariant()
        executable_input_review_sha256 = (Get-FileHash -LiteralPath $absoluteExecutableReviewPath -Algorithm SHA256).Hash.ToLowerInvariant()
        proc_macro_runtime_review_sha256 = (Get-FileHash -LiteralPath $absoluteProcMacroReviewPath -Algorithm SHA256).Hash.ToLowerInvariant()
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
    packages = @($catalog | Sort-Object { $_.id })
    risk_classification = [ordered]@{
        status = [string]$riskDocument.status
        change_rule = [string]$riskDocument.change_rule
        classified_package_count = @($normalizedRiskClassifications).Count
        unclassified_package_count = @($catalog).Count - @($normalizedRiskClassifications).Count
        required_direct_core_normal_count = $directCoreNormalIds.Count
        entries = @($normalizedRiskClassifications | Sort-Object { $_.id })
        transitive_exposure_count = @($normalizedRiskExposure).Count
        transitive_exposure = @($normalizedRiskExposure | Sort-Object { $_.id })
    }
    profiles = @($profiles)
    core_artifact_profiles = @($coreArtifactProfiles)
    build_script_review = [ordered]@{
        status = [string]$buildReviewDocument.status
        scope = [string]$buildReviewDocument.scope
        reviewed_candidate_count = @($normalizedBuildReviews).Count
        entries = @($normalizedBuildReviews | Sort-Object { $_.id })
    }
    executable_input_review = [ordered]@{
        status = [string]$executableReviewDocument.status
        reviewed_subject_count = @($normalizedExecutableReviews).Count
        entries = @($normalizedExecutableReviews | Sort-Object { $_.id })
    }
    proc_macro_runtime_review = [ordered]@{
        status = [string]$procMacroReviewDocument.status
        method = [string]$procMacroReviewDocument.method
        reviewed_package_count = @($normalizedProcMacroReviews).Count
        entries = @($normalizedProcMacroReviews | Sort-Object { $_.id })
    }
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
