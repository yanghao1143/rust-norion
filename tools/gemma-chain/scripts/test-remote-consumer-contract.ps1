param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$SnapshotJsonPath = "",
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Read-Snapshot {
    param(
        [string]$Root,
        [string]$Path
    )

    if (-not [string]::IsNullOrWhiteSpace($Path)) {
        return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
    }

    $snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"
    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $snapshotScript -RepoRoot $Root -Json
    if ($LASTEXITCODE -ne 0) {
        throw "snapshot script exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$snapshot = Read-Snapshot -Root $root -Path $SnapshotJsonPath

$expectedConsumerIds = @(
    "web_lab_prompt",
    "forge_cli_prompt",
    "backend_cli_direct_prompt",
    "evolution_loop_prompt_round",
    "model_pool_launch",
    "forge_daemon_residency",
    "ssh_remote_probe"
)

Assert-True ($snapshot.contract_version -eq "smartsteam.remote-gemma-unattended.snapshot-summary.v1") "snapshot contract version mismatch"
Assert-True ($snapshot.read_only -eq $true -and $snapshot.starts_process -eq $false -and $snapshot.sends_prompt -eq $false -and $snapshot.touches_remote -eq $false -and $snapshot.writes_files -eq $false) "snapshot must be read-only"
Assert-True ($snapshot.authorization.can_authorize_daemon -eq $false -and $snapshot.authorization.can_authorize_launch -eq $false -and $snapshot.authorization.can_authorize_prompt -eq $false -and $snapshot.authorization.can_authorize_ssh -eq $false) "snapshot authorization must fail closed"
Assert-True ($snapshot.consumer_contract.contract_version -eq "smartsteam.remote-gemma-unattended.consumer-projection.v1") "consumer contract version mismatch"
Assert-True ($snapshot.consumer_contract.fail_closed_default -eq $true) "consumer contract must default fail closed"
Assert-True ($snapshot.consumer_contract.allowed_requires_external_gates -eq $true) "consumer contract must require external gates"

$requiredFields = @($snapshot.consumer_contract.required_fields)
$supportedKinds = @($snapshot.consumer_contract.supported_entrypoint_kinds)
$projection = @($snapshot.consumer_projection)
$safeCommands = @($snapshot.safe_next_read_only_commands)

Assert-True ($projection.Count -eq $expectedConsumerIds.Count) "consumer projection count mismatch"
Assert-True (@($projection | Group-Object id | Where-Object { $_.Count -ne 1 }).Count -eq 0) "consumer ids must be unique"

foreach ($expectedId in $expectedConsumerIds) {
    Assert-True (@($projection | Where-Object { $_.id -eq $expectedId }).Count -eq 1) "missing consumer $expectedId"
    Assert-True (@($snapshot.consumer_contract.consumer_ids | Where-Object { $_ -eq $expectedId }).Count -eq 1) "consumer contract missing id $expectedId"
}

foreach ($command in $safeCommands) {
    Assert-True ($command.read_only -eq $true) "safe command $($command.id) must be read_only"
    Assert-True ($command.starts_process -eq $false) "safe command $($command.id) must not start processes"
    Assert-True ($command.sends_prompt -eq $false) "safe command $($command.id) must not send prompts"
    Assert-True ($command.touches_remote -eq $false) "safe command $($command.id) must not touch remote"
    Assert-True ($command.writes_files -eq $false) "safe command $($command.id) must not write files"
}

foreach ($consumer in $projection) {
    foreach ($field in $requiredFields) {
        Assert-True ($null -ne $consumer.PSObject.Properties[$field]) "consumer $($consumer.id) missing required field $field"
    }

    Assert-True (@($supportedKinds | Where-Object { $_ -eq $consumer.entrypoint_kind }).Count -eq 1) "consumer $($consumer.id) has unsupported entrypoint kind"
    Assert-True ($consumer.current_allowed -eq $false) "consumer $($consumer.id) must fail closed in snapshot-only output"
    Assert-True (@($consumer.blocked_by).Count -gt 0) "consumer $($consumer.id) must expose blocked_by"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$consumer.safe_command_id)) "consumer $($consumer.id) missing safe_command_id"
    Assert-True (@($safeCommands | Where-Object { $_.id -eq $consumer.safe_command_id }).Count -eq 1) "consumer $($consumer.id) safe_command_id must exist"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$consumer.reason)) "consumer $($consumer.id) missing reason"

    if ($consumer.entrypoint_kind -eq "prompt") {
        Assert-True ($consumer.downstream_sends_prompt -eq $true) "prompt consumer $($consumer.id) must mark downstream_sends_prompt"
        Assert-True ($consumer.downstream_launches_process -eq $false) "prompt consumer $($consumer.id) should not mark launch"
        Assert-True ($consumer.downstream_touches_remote -eq $false) "prompt consumer $($consumer.id) should not mark remote touch"
    } elseif ($consumer.entrypoint_kind -eq "launch") {
        Assert-True ($consumer.downstream_sends_prompt -eq $false) "launch consumer $($consumer.id) should not mark prompt"
        Assert-True ($consumer.downstream_launches_process -eq $true) "launch consumer $($consumer.id) must mark process launch"
        Assert-True ($consumer.downstream_touches_remote -eq $false) "launch consumer $($consumer.id) should not mark remote touch"
    } elseif ($consumer.entrypoint_kind -eq "ssh") {
        Assert-True ($consumer.downstream_sends_prompt -eq $false) "ssh consumer $($consumer.id) should not mark prompt"
        Assert-True ($consumer.downstream_launches_process -eq $false) "ssh consumer $($consumer.id) should not mark launch"
        Assert-True ($consumer.downstream_touches_remote -eq $true) "ssh consumer $($consumer.id) must mark remote touch"
    }
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.consumer-contract-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    repo_root = $root
    summary = [pscustomobject]@{
        consumer_contract_version = $snapshot.consumer_contract.contract_version
        consumer_count = $projection.Count
        consumer_allowed_count = @($projection | Where-Object { $_.current_allowed -eq $true }).Count
        prompt_consumer_count = @($projection | Where-Object { $_.entrypoint_kind -eq "prompt" }).Count
        launch_consumer_count = @($projection | Where-Object { $_.entrypoint_kind -eq "launch" }).Count
        ssh_consumer_count = @($projection | Where-Object { $_.entrypoint_kind -eq "ssh" }).Count
        safe_command_count = $safeCommands.Count
        invalid_safe_command_count = @($safeCommands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count
        fail_closed_default = $snapshot.consumer_contract.fail_closed_default
        allowed_requires_external_gates = $snapshot.consumer_contract.allowed_requires_external_gates
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "consumer_contract_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 6
    exit 0
}

Write-Host "Gemma remote consumer contract selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False"
Write-Host "contract=$($result.summary.consumer_contract_version) consumers=$($result.summary.consumer_count) allowed=$($result.summary.consumer_allowed_count)"
Write-Host "prompt=$($result.summary.prompt_consumer_count) launch=$($result.summary.launch_consumer_count) ssh=$($result.summary.ssh_consumer_count)"
Write-Host "safe_commands=$($result.summary.safe_command_count) invalid_safe_commands=$($result.summary.invalid_safe_command_count)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
