param(
    [string]$RepoRoot = "",
    [string]$Ledger = "target\evolution\evolution-ledger.jsonl",
    [string]$ReportJson = "",
    [string]$Backend = "127.0.0.1:7979",
    [switch]$SkipBackend,
    [string]$RemoteChainStatusJson = "target\remote-gemma-chain\status-with-model-cache.json",
    [switch]$SkipRemoteChain,
    [switch]$SkipProcess,
    [switch]$SkipDaemon,
    [string]$DaemonWorkDir = "target\evolution\daemon",
    [switch]$UseDaemonLedger,
    [switch]$RequireDaemonHealthy,
    [switch]$RequireDaemonValidationExecution,
    [switch]$RequireLatestConfiguredValidationRun,
    [switch]$RequireLatestSelfImprove,
    [string]$RequireLatestHelperStageRoles = "",
    [switch]$RequireLatestHelperStageContracts,
    [switch]$RequireLatestTestGatePass,
    [switch]$RequireLatestSafeTestGateValidationCommand,
    [switch]$StrictUnattendedEvolution,
    [switch]$StrictLedgerHygiene,
    [int]$MinRounds = 1,
    [int]$MinFeedbackTotal = 1,
    [int]$MaxDaemonInProgressStdoutAgeSeconds = 300,
    [int]$MaxDaemonRoundTimeoutSeconds = 900,
    [int]$MaxDaemonIdleLedgerAgeSeconds = 0,
    [string]$BackendHealthJson = "",
    [string]$BackendHealthJsonPath = "",
    [switch]$JsonStatus,
    [switch]$FailOnNotReady,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($RepoRoot.Trim().Length -eq 0) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
}

if ($Help) {
    Write-Host "Read SmartSteam evolution-loop status without starting processes or sending prompts."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-JsonStatus] [-Ledger PATH]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-DaemonWorkDir PATH] [-SkipDaemon]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-UseDaemonLedger] [-RequireDaemonHealthy]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireDaemonHealthy] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireDaemonValidationExecution] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireLatestConfiguredValidationRun] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireLatestSelfImprove] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireLatestHelperStageRoles summary,router,review,index,test-gate] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireLatestHelperStageContracts] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-RequireLatestTestGatePass] [-RequireLatestSafeTestGateValidationCommand] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-StrictUnattendedEvolution] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\status-evolution-loop.cmd [-MaxDaemonInProgressStdoutAgeSeconds 300] [-MaxDaemonRoundTimeoutSeconds 900] [-MaxDaemonIdleLedgerAgeSeconds 900] [-FailOnNotReady]"
    Write-Host ""
    Write-Host "Contracts:"
    Write-Host "  read_only=true"
    Write-Host "  starts_process=false"
    Write-Host "  sends_prompt=false"
    exit 0
}

$DefaultStrictHelperStageRoles = "summary,router,review,index,test-gate"
$UseDaemonLedgerEffective = [bool]$UseDaemonLedger -or [bool]$StrictUnattendedEvolution
$RequireDaemonHealthyEffective = [bool]$RequireDaemonHealthy -or [bool]$StrictUnattendedEvolution
$RequireDaemonValidationExecutionEffective = [bool]$RequireDaemonValidationExecution -or [bool]$StrictUnattendedEvolution
$RequireLatestConfiguredValidationRunEffective = [bool]$RequireLatestConfiguredValidationRun -or [bool]$StrictUnattendedEvolution
$RequireLatestSelfImproveEffective = [bool]$RequireLatestSelfImprove -or [bool]$StrictUnattendedEvolution
$RequireLatestHelperStageRolesEffective = if ($StrictUnattendedEvolution -and $RequireLatestHelperStageRoles.Trim().Length -eq 0) { $DefaultStrictHelperStageRoles } else { $RequireLatestHelperStageRoles }
$RequireLatestHelperStageContractsEffective = [bool]$RequireLatestHelperStageContracts -or [bool]$StrictUnattendedEvolution
$RequireLatestTestGatePassEffective = [bool]$RequireLatestTestGatePass -or [bool]$StrictUnattendedEvolution
$RequireLatestSafeTestGateValidationCommandEffective = [bool]$RequireLatestSafeTestGateValidationCommand -or [bool]$StrictUnattendedEvolution
$MaxDaemonInProgressStdoutAgeSecondsEffective = if ($StrictUnattendedEvolution -and -not $PSBoundParameters.ContainsKey("MaxDaemonInProgressStdoutAgeSeconds")) { 900 } else { $MaxDaemonInProgressStdoutAgeSeconds }
$MaxDaemonRoundTimeoutSecondsEffective = if ($StrictUnattendedEvolution -and -not $PSBoundParameters.ContainsKey("MaxDaemonRoundTimeoutSeconds")) { 900 } else { $MaxDaemonRoundTimeoutSeconds }
$MaxDaemonIdleLedgerAgeSecondsEffective = if ($StrictUnattendedEvolution -and -not $PSBoundParameters.ContainsKey("MaxDaemonIdleLedgerAgeSeconds")) { 900 } else { $MaxDaemonIdleLedgerAgeSeconds }

function Resolve-RepoPath {
    param([string]$Path)

    if ($Path.Trim().Length -eq 0) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $RepoRoot $Path)
}

function Has-Property {
    param(
        [object]$Object,
        [string]$Name
    )

    return $null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name
}

function Get-PropertyValue {
    param(
        [object]$Object,
        [string]$Name
    )

    if (Has-Property $Object $Name) {
        return $Object.$Name
    }
    return $null
}

function Get-NestedValue {
    param(
        [object]$Value,
        [string[]]$Path
    )

    $cursor = $Value
    foreach ($segment in $Path) {
        if ($null -eq $cursor -or -not (Has-Property -Object $cursor -Name $segment)) {
            return $null
        }
        $cursor = $cursor.$segment
    }
    return $cursor
}

function Convert-ToPositiveInt {
    param([object]$Value)

    if ($null -eq $Value) {
        return 0
    }
    try {
        $number = [int64]$Value
        if ($number -gt 0 -and $number -le [int64][int]::MaxValue) {
            return [int]$number
        }
    } catch {
        return 0
    }
    return 0
}

function Convert-ToNullableInt {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    try {
        return [int]([int64]$Value)
    } catch {
        return $null
    }
}

function Convert-ToNullableDouble {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    try {
        return [double]$Value
    } catch {
        return $null
    }
}

function Convert-ToNullableBool {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    $text = ([string]$Value).Trim()
    if ($text.Equals("true", [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    if ($text.Equals("false", [System.StringComparison]::OrdinalIgnoreCase)) {
        return $false
    }
    return $null
}

function Get-SelfImproveProposalAcceptanceSummary {
    param([object]$Report)

    $topLevelActionAssignment = Resolve-SelfImproveProposalActionAssignment `
        -ActionAssignment (Get-PropertyValue -Object $Report -Name "self_improve_proposal_action_assignment_v1") `
        -Source "self_improve_proposal_action_assignment_v1"
    $summary = Get-PropertyValue -Object $Report -Name "self_improve_proposal_acceptance_summary_v1"
    if ($null -ne $summary) {
        $business = Convert-ToNullableInt (Get-PropertyValue -Object $summary -Name "evidence_backed_business_improvement_count")
        $advisory = Convert-ToNullableInt (Get-PropertyValue -Object $summary -Name "advisory_only_count")
        $repair = Convert-ToNullableInt (Get-PropertyValue -Object $summary -Name "require_repair_count")
        $acceptedWithoutEvidence = Convert-ToNullableInt (Get-PropertyValue -Object $summary -Name "accepted_without_business_evidence_count")
        if ($null -ne $business -or $null -ne $advisory -or $null -ne $repair -or $null -ne $acceptedWithoutEvidence) {
            $guidance = Get-PropertyValue -Object $summary -Name "prompt_guidance"
            $convert = Resolve-SelfImproveProposalGuidanceBool -Value (Convert-ToNullableBool (Get-PropertyValue -Object $guidance -Name "should_convert_advisory_to_evidence_backed_business_improvement")) -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "convert"
            $repairGuidance = Resolve-SelfImproveProposalGuidanceBool -Value (Convert-ToNullableBool (Get-PropertyValue -Object $guidance -Name "should_repair_unvalidated_or_unaccepted_proposals")) -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "repair"
            $requiresValidation = Resolve-SelfImproveProposalGuidanceBool -Value (Convert-ToNullableBool (Get-PropertyValue -Object $guidance -Name "requires_checked_passed_validation_and_accepted_memory_admission")) -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "requires_validation"
            $actionPlan = Resolve-SelfImproveProposalActionPlan -ActionPlan (Get-PropertyValue -Object $summary -Name "action_plan") -ConvertAdvisory $convert -RepairUnvalidated $repairGuidance -RequiresValidation $requiresValidation -Available $true
            $nestedActionAssignment = Resolve-SelfImproveProposalActionAssignment `
                -ActionAssignment (Get-PropertyValue -Object $summary -Name "action_assignment") `
                -Source "self_improve_proposal_acceptance_summary_v1.action_assignment"
            $actionAssignment = Select-SelfImproveProposalActionAssignment -Preferred $topLevelActionAssignment -Fallback $nestedActionAssignment
            return [pscustomobject][ordered]@{
                source = "self_improve_proposal_acceptance_summary_v1"
                evidence_backed_business_improvement_count = $business
                advisory_only_count = $advisory
                require_repair_count = $repair
                accepted_without_business_evidence_count = $acceptedWithoutEvidence
                should_convert_advisory_to_evidence_backed_business_improvement = $convert
                should_repair_unvalidated_or_unaccepted_proposals = $repairGuidance
                requires_checked_passed_validation_and_accepted_memory_admission = $requiresValidation
                action_required = Select-FirstNonNull -Preferred $actionAssignment.action_required -Fallback $actionPlan.action_required
                primary_action = Select-FirstNonEmptyString -Preferred $actionAssignment.primary_action -Fallback $actionPlan.primary_action
                actions = Select-FirstNonEmptyList -Preferred $actionAssignment.actions -Fallback $actionPlan.actions
                action_plan_requires_checked_passed_validation_and_accepted_memory_admission = Select-FirstNonNull -Preferred $actionAssignment.requires_checked_passed_validation_and_accepted_memory_admission -Fallback $actionPlan.requires_checked_passed_validation_and_accepted_memory_admission
                action_assignment_source = $actionAssignment.source
                action_assignment_target_count = $actionAssignment.target_count
                action_assignment_first_target = $actionAssignment.first_target
                action_assignment_first_missing_requirements = $actionAssignment.first_missing_requirements
            }
        }
    }

    $artifact = Get-PropertyValue -Object $Report -Name "self_improve_proposal_artifact_v1"
    if ($null -eq $artifact) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            evidence_backed_business_improvement_count = $null
            advisory_only_count = $null
            require_repair_count = $null
            accepted_without_business_evidence_count = $null
            should_convert_advisory_to_evidence_backed_business_improvement = $null
            should_repair_unvalidated_or_unaccepted_proposals = $null
            requires_checked_passed_validation_and_accepted_memory_admission = $null
            action_required = $topLevelActionAssignment.action_required
            primary_action = $topLevelActionAssignment.primary_action
            actions = $topLevelActionAssignment.actions
            action_plan_requires_checked_passed_validation_and_accepted_memory_admission = $topLevelActionAssignment.requires_checked_passed_validation_and_accepted_memory_admission
            action_assignment_source = $topLevelActionAssignment.source
            action_assignment_target_count = $topLevelActionAssignment.target_count
            action_assignment_first_target = $topLevelActionAssignment.first_target
            action_assignment_first_missing_requirements = $topLevelActionAssignment.first_missing_requirements
        }
    }

    $business = 0
    $advisory = 0
    $repair = 0
    $acceptedWithoutEvidence = 0
    foreach ($proposal in @((Get-PropertyValue -Object $artifact -Name "proposals") | Where-Object { $null -ne $_ })) {
        $acceptance = Get-PropertyValue -Object $proposal -Name "business_improvement_acceptance"
        if ($null -eq $acceptance) {
            continue
        }
        $isBusiness = Convert-ToNullableBool (Get-PropertyValue -Object $acceptance -Name "evidence_backed_business_improvement")
        $isAdvisory = Convert-ToNullableBool (Get-PropertyValue -Object $acceptance -Name "advisory_only")
        $needsRepair = Convert-ToNullableBool (Get-PropertyValue -Object $acceptance -Name "require_repair")
        $memoryAccepted = Convert-ToNullableBool (Get-PropertyValue -Object $acceptance -Name "memory_admission_accepted")
        if ($isBusiness -eq $true) {
            $business += 1
        }
        if ($isAdvisory -eq $true) {
            $advisory += 1
        }
        if ($needsRepair -eq $true) {
            $repair += 1
        }
        if ($memoryAccepted -eq $true -and $isBusiness -ne $true) {
            $acceptedWithoutEvidence += 1
        }
    }

    $convert = Resolve-SelfImproveProposalGuidanceBool -Value $null -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "convert"
    $repairGuidance = Resolve-SelfImproveProposalGuidanceBool -Value $null -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "repair"
    $requiresValidation = Resolve-SelfImproveProposalGuidanceBool -Value $null -Business $business -Advisory $advisory -Repair $repair -AcceptedWithoutEvidence $acceptedWithoutEvidence -Name "requires_validation"
    $actionPlan = Resolve-SelfImproveProposalActionPlan -ActionPlan $null -ConvertAdvisory $convert -RepairUnvalidated $repairGuidance -RequiresValidation $requiresValidation -Available $true

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_artifact_v1"
        evidence_backed_business_improvement_count = $business
        advisory_only_count = $advisory
        require_repair_count = $repair
        accepted_without_business_evidence_count = $acceptedWithoutEvidence
        should_convert_advisory_to_evidence_backed_business_improvement = $convert
        should_repair_unvalidated_or_unaccepted_proposals = $repairGuidance
        requires_checked_passed_validation_and_accepted_memory_admission = $requiresValidation
        action_required = Select-FirstNonNull -Preferred $topLevelActionAssignment.action_required -Fallback $actionPlan.action_required
        primary_action = Select-FirstNonEmptyString -Preferred $topLevelActionAssignment.primary_action -Fallback $actionPlan.primary_action
        actions = Select-FirstNonEmptyList -Preferred $topLevelActionAssignment.actions -Fallback $actionPlan.actions
        action_plan_requires_checked_passed_validation_and_accepted_memory_admission = Select-FirstNonNull -Preferred $topLevelActionAssignment.requires_checked_passed_validation_and_accepted_memory_admission -Fallback $actionPlan.requires_checked_passed_validation_and_accepted_memory_admission
        action_assignment_source = $topLevelActionAssignment.source
        action_assignment_target_count = $topLevelActionAssignment.target_count
        action_assignment_first_target = $topLevelActionAssignment.first_target
        action_assignment_first_missing_requirements = $topLevelActionAssignment.first_missing_requirements
    }
}

function Resolve-SelfImproveProposalActionAssignment {
    param(
        [object]$ActionAssignment,
        [string]$Source = ""
    )

    if ($null -eq $ActionAssignment) {
        return [pscustomobject][ordered]@{
            available = $false
            source = "unavailable"
            action_required = $null
            primary_action = $null
            actions = @()
            requires_checked_passed_validation_and_accepted_memory_admission = $null
            target_count = $null
            first_target = $null
            first_missing_requirements = @()
        }
    }

    $targets = @((Get-PropertyValue -Object $ActionAssignment -Name "targets") | Where-Object { $null -ne $_ })
    $targetCount = Convert-ToNullableInt (Get-PropertyValue -Object $ActionAssignment -Name "target_count")
    if ($null -eq $targetCount) {
        $targetCount = [int]$targets.Count
    }
    $firstTarget = $null
    $firstMissing = @()
    $firstTargetObject = Get-PropertyValue -Object $ActionAssignment -Name "first_target"
    if ($null -eq $firstTargetObject -and $targets.Count -gt 0) {
        $firstTargetObject = $targets[0]
    }
    if ($null -ne $firstTargetObject) {
        $firstTarget = Get-PropertyValue -Object $firstTargetObject -Name "proposal_id"
        $firstMissing = Convert-ToStringList (Get-PropertyValue -Object $firstTargetObject -Name "missing_requirements")
    }

    return [pscustomobject][ordered]@{
        available = $true
        source = if ([string]::IsNullOrWhiteSpace($Source)) { "self_improve_proposal_action_assignment" } else { $Source }
        action_required = Convert-ToNullableBool (Get-PropertyValue -Object $ActionAssignment -Name "action_required")
        primary_action = Get-PropertyValue -Object $ActionAssignment -Name "primary_action"
        actions = Convert-ToStringList (Get-PropertyValue -Object $ActionAssignment -Name "actions")
        requires_checked_passed_validation_and_accepted_memory_admission = Convert-ToNullableBool (Get-PropertyValue -Object $ActionAssignment -Name "requires_checked_passed_validation_and_accepted_memory_admission")
        target_count = $targetCount
        first_target = $firstTarget
        first_missing_requirements = $firstMissing
    }
}

function Select-SelfImproveProposalActionAssignment {
    param(
        [object]$Preferred,
        [object]$Fallback
    )

    if ($null -ne $Preferred -and $Preferred.available -eq $true) {
        return $Preferred
    }
    if ($null -ne $Fallback) {
        return $Fallback
    }
    return Resolve-SelfImproveProposalActionAssignment -ActionAssignment $null
}

function Get-SelfImproveProposalRepairFactorReadiness {
    param([object]$Report)

    $readiness = Get-PropertyValue -Object $Report -Name "self_improve_proposal_repair_factor_readiness_report_v1"
    if ($null -eq $readiness) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            action_required = $null
            repair_factor_count = $null
            ready_repair_factor_count = $null
            blocked_count = $null
            all_repair_factors_ready = $null
            first_repair_factor_id = $null
            first_repair_factor_ready = $null
            first_repair_factor_status = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_repair_factor_readiness_report_v1"
        action_required = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "action_required")
        repair_factor_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "repair_factor_count")
        ready_repair_factor_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "ready_repair_factor_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "blocked_count")
        all_repair_factors_ready = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "all_repair_factors_ready")
        first_repair_factor_id = Get-PropertyValue -Object $readiness -Name "first_repair_factor_id"
        first_repair_factor_ready = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "first_repair_factor_ready")
        first_repair_factor_status = Get-PropertyValue -Object $readiness -Name "first_repair_factor_status"
    }
}

function Get-SelfImproveProposalRepairFactorRelease {
    param([object]$Report)

    $release = Get-PropertyValue -Object $Report -Name "self_improve_proposal_repair_factor_release_report_v1"
    if ($null -eq $release) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            action_required = $null
            repair_factor_count = $null
            release_count = $null
            blocked_count = $null
            release_ready = $null
            first_repair_factor_id = $null
            first_release_ready = $null
            first_release_status = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_repair_factor_release_report_v1"
        action_required = Convert-ToNullableBool (Get-PropertyValue -Object $release -Name "action_required")
        repair_factor_count = Convert-ToNullableInt (Get-PropertyValue -Object $release -Name "repair_factor_count")
        release_count = Convert-ToNullableInt (Get-PropertyValue -Object $release -Name "release_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $release -Name "blocked_count")
        release_ready = Convert-ToNullableBool (Get-PropertyValue -Object $release -Name "release_ready")
        first_repair_factor_id = Get-PropertyValue -Object $release -Name "first_repair_factor_id"
        first_release_ready = Convert-ToNullableBool (Get-PropertyValue -Object $release -Name "first_release_ready")
        first_release_status = Get-PropertyValue -Object $release -Name "first_release_status"
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $release -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $release -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalRepairFactorRetagPlan {
    param([object]$Report)

    $retag = Get-PropertyValue -Object $Report -Name "self_improve_proposal_repair_factor_retag_plan_v1"
    if ($null -eq $retag) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            action_required = $null
            repair_factor_count = $null
            retag_plan_count = $null
            blocked_count = $null
            retag_plan_ready = $null
            first_repair_factor_id = $null
            first_retag_ready = $null
            first_retag_status = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_repair_factor_retag_plan_v1"
        action_required = Convert-ToNullableBool (Get-PropertyValue -Object $retag -Name "action_required")
        repair_factor_count = Convert-ToNullableInt (Get-PropertyValue -Object $retag -Name "repair_factor_count")
        retag_plan_count = Convert-ToNullableInt (Get-PropertyValue -Object $retag -Name "retag_plan_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $retag -Name "blocked_count")
        retag_plan_ready = Convert-ToNullableBool (Get-PropertyValue -Object $retag -Name "retag_plan_ready")
        first_repair_factor_id = Get-PropertyValue -Object $retag -Name "first_repair_factor_id"
        first_retag_ready = Convert-ToNullableBool (Get-PropertyValue -Object $retag -Name "first_retag_ready")
        first_retag_status = Get-PropertyValue -Object $retag -Name "first_retag_status"
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $retag -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $retag -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalActionClosure {
    param([object]$Report)

    $closure = Get-PropertyValue -Object $Report -Name "self_improve_proposal_action_closure_report_v1"
    if ($null -eq $closure) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            closed_target_count = $null
            open_target_count = $null
            first_target = $null
            first_target_closed = $null
            first_target_closure_kind = $null
            first_target_still_requires_memory_admission = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_action_closure_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $closure -Name "target_count")
        closed_target_count = Convert-ToNullableInt (Get-PropertyValue -Object $closure -Name "closed_target_count")
        open_target_count = Convert-ToNullableInt (Get-PropertyValue -Object $closure -Name "open_target_count")
        first_target = Get-PropertyValue -Object $closure -Name "first_target_id"
        first_target_closed = Convert-ToNullableBool (Get-PropertyValue -Object $closure -Name "first_target_closed")
        first_target_closure_kind = Get-PropertyValue -Object $closure -Name "first_target_closure_kind"
        first_target_still_requires_memory_admission = Convert-ToNullableBool (Get-PropertyValue -Object $closure -Name "first_target_still_requires_memory_admission")
    }
}

function Get-SelfImproveProposalMemoryAdmissionReadiness {
    param([object]$Report)

    $readiness = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_readiness_report_v1"
    if ($null -eq $readiness) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            ready_count = $null
            blocked_count = $null
            first_target = $null
            first_target_ready = $null
            all_closed_targets_ready = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_readiness_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "target_count")
        ready_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "ready_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $readiness -Name "blocked_count")
        first_target = Get-PropertyValue -Object $readiness -Name "first_target_id"
        first_target_ready = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "first_target_ready")
        all_closed_targets_ready = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "all_closed_targets_ready")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $readiness -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionRequest {
    param([object]$Report)

    $request = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_request_report_v1"
    if ($null -eq $request) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            blocked_count = $null
            first_candidate = $null
            first_candidate_ready = $null
            all_ready_targets_requested = $null
            writer_required = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_request_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "request_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "blocked_count")
        first_candidate = Get-PropertyValue -Object $request -Name "first_candidate_id"
        first_candidate_ready = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "first_candidate_ready")
        all_ready_targets_requested = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "all_ready_targets_requested")
        writer_required = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "writer_required")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionDecision {
    param([object]$Report)

    $decision = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_decision_report_v1"
    if ($null -eq $decision) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            blocked_count = $null
            first_candidate = $null
            writer_required = $null
            admission_writer_preflight_passed = $null
            explicit_writer_invocation_required = $null
            admission_write_authorized = $null
            gate_blocked = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_decision_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "request_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "blocked_count")
        first_candidate = Get-PropertyValue -Object $decision -Name "first_candidate_id"
        writer_required = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "writer_required")
        admission_writer_preflight_passed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "admission_writer_preflight_passed")
        explicit_writer_invocation_required = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "explicit_writer_invocation_required")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "admission_write_authorized")
        gate_blocked = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "gate_blocked")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $decision -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionWriterPlan {
    param([object]$Report)

    $writerPlan = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_writer_plan_report_v1"
    if ($null -eq $writerPlan) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            writer_plan_item_count = $null
            ready_plan_count = $null
            blocked_count = $null
            first_plan_item = $null
            writer_plan_ready = $null
            explicit_writer_invocation_required = $null
            experiment_required = $null
            rollback_required = $null
            validation_required = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_writer_plan_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $writerPlan -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $writerPlan -Name "request_count")
        writer_plan_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $writerPlan -Name "writer_plan_item_count")
        ready_plan_count = Convert-ToNullableInt (Get-PropertyValue -Object $writerPlan -Name "ready_plan_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $writerPlan -Name "blocked_count")
        first_plan_item = Get-PropertyValue -Object $writerPlan -Name "first_plan_item_id"
        writer_plan_ready = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "writer_plan_ready")
        explicit_writer_invocation_required = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "explicit_writer_invocation_required")
        experiment_required = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "experiment_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "rollback_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "validation_required")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $writerPlan -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $writerPlan -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionWriterDryRun {
    param([object]$Report)

    $dryRun = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_writer_dry_run_report_v1"
    if ($null -eq $dryRun) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            writer_plan_item_count = $null
            dry_run_item_count = $null
            ready_dry_run_count = $null
            blocked_count = $null
            first_dry_run_item = $null
            dry_run_ready = $null
            explicit_writer_invocation_required = $null
            dry_run_required = $null
            experiment_required = $null
            rollback_required = $null
            validation_required = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_writer_dry_run_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "request_count")
        writer_plan_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "writer_plan_item_count")
        dry_run_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "dry_run_item_count")
        ready_dry_run_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "ready_dry_run_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $dryRun -Name "blocked_count")
        first_dry_run_item = Get-PropertyValue -Object $dryRun -Name "first_dry_run_item_id"
        dry_run_ready = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "dry_run_ready")
        explicit_writer_invocation_required = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "explicit_writer_invocation_required")
        dry_run_required = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "dry_run_required")
        experiment_required = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "experiment_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "rollback_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "validation_required")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $dryRun -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $dryRun -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionWriterDryRunReceipt {
    param([object]$Report)

    $receipt = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1"
    if ($null -eq $receipt) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            dry_run_item_count = $null
            receipt_item_count = $null
            succeeded_receipt_count = $null
            blocked_count = $null
            first_receipt_item = $null
            dry_run_receipt_ready = $null
            explicit_writer_invocation_required = $null
            commit_allowed = $null
            validation_required = $null
            rollback_required = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "request_count")
        dry_run_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "dry_run_item_count")
        receipt_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "receipt_item_count")
        succeeded_receipt_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "succeeded_receipt_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $receipt -Name "blocked_count")
        first_receipt_item = Get-PropertyValue -Object $receipt -Name "first_receipt_item_id"
        dry_run_receipt_ready = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "dry_run_receipt_ready")
        explicit_writer_invocation_required = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "explicit_writer_invocation_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "commit_allowed")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "rollback_required")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $receipt -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $receipt -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionCommitRecordStage {
    param([object]$Report)

    $stage = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_commit_record_stage_report_v1"
    if ($null -eq $stage) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            receipt_item_count = $null
            commit_record_item_count = $null
            staged_commit_record_count = $null
            blocked_count = $null
            first_commit_record_item = $null
            commit_record_stage_ready = $null
            explicit_writer_invocation_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_commit_record_stage_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "request_count")
        receipt_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "receipt_item_count")
        commit_record_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "commit_record_item_count")
        staged_commit_record_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "staged_commit_record_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $stage -Name "blocked_count")
        first_commit_record_item = Get-PropertyValue -Object $stage -Name "first_commit_record_item_id"
        commit_record_stage_ready = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "commit_record_stage_ready")
        explicit_writer_invocation_required = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "explicit_writer_invocation_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $stage -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $stage -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionCommitApprovalRequest {
    param([object]$Report)

    $request = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_commit_approval_request_report_v1"
    if ($null -eq $request) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            commit_record_item_count = $null
            approval_request_item_count = $null
            requested_commit_approval_count = $null
            blocked_count = $null
            first_approval_request_item = $null
            commit_approval_request_ready = $null
            explicit_commit_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_commit_approval_request_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "request_count")
        commit_record_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "commit_record_item_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "approval_request_item_count")
        requested_commit_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "requested_commit_approval_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $request -Name "blocked_count")
        first_approval_request_item = Get-PropertyValue -Object $request -Name "first_approval_request_item_id"
        commit_approval_request_ready = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "commit_approval_request_ready")
        explicit_commit_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "explicit_commit_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $request -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $request -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionCommitApprovalDecision {
    param([object]$Report)

    $decision = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_commit_approval_decision_report_v1"
    if ($null -eq $decision) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            approval_request_item_count = $null
            approval_decision_item_count = $null
            recorded_approval_decision_count = $null
            approved_commit_count = $null
            pending_approval_count = $null
            blocked_count = $null
            first_approval_decision_item = $null
            commit_approval_decision_ready = $null
            explicit_commit_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_commit_approval_decision_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "request_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "approval_request_item_count")
        approval_decision_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "approval_decision_item_count")
        recorded_approval_decision_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "recorded_approval_decision_count")
        approved_commit_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "approved_commit_count")
        pending_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "pending_approval_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $decision -Name "blocked_count")
        first_approval_decision_item = Get-PropertyValue -Object $decision -Name "first_approval_decision_item_id"
        commit_approval_decision_ready = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "commit_approval_decision_ready")
        explicit_commit_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "explicit_commit_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $decision -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decision -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacket {
    param([object]$Report)

    $review = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1"
    if ($null -eq $review) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            request_count = $null
            approval_request_item_count = $null
            approval_decision_item_count = $null
            review_packet_item_count = $null
            ready_review_packet_count = $null
            pending_approval_count = $null
            blocked_count = $null
            first_review_packet_item = $null
            approval_review_packet_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "target_count")
        request_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "request_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "approval_request_item_count")
        approval_decision_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "approval_decision_item_count")
        review_packet_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "review_packet_item_count")
        ready_review_packet_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "ready_review_packet_count")
        pending_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "pending_approval_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $review -Name "blocked_count")
        first_review_packet_item = Get-PropertyValue -Object $review -Name "first_review_packet_item_id"
        approval_review_packet_ready = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "approval_review_packet_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $review -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $review -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionUsefulness {
    param([object]$Report)

    $reflection = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_usefulness_report_v1"
    if ($null -eq $reflection) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            projected_report_count = $null
            accepted_memory_admission_count = $null
            quarantined_candidate_count = $null
            review_packet_item_count = $null
            useful_reflection_item_count = $null
            pending_operator_approval_count = $null
            blocked_count = $null
            wasted_compute_guard_count = $null
            adapter_safe_count = $null
            first_reflection_item = $null
            reflection_usefulness_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_usefulness_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "target_count")
        projected_report_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "projected_report_count")
        accepted_memory_admission_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "accepted_memory_admission_count")
        quarantined_candidate_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "quarantined_candidate_count")
        review_packet_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "review_packet_item_count")
        useful_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "useful_reflection_item_count")
        pending_operator_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "pending_operator_approval_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "blocked_count")
        wasted_compute_guard_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "wasted_compute_guard_count")
        adapter_safe_count = Convert-ToNullableInt (Get-PropertyValue -Object $reflection -Name "adapter_safe_count")
        first_reflection_item = Get-PropertyValue -Object $reflection -Name "first_reflection_item_id"
        reflection_usefulness_ready = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "reflection_usefulness_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $reflection -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reflection -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionDedupeCluster {
    param([object]$Report)

    $dedupe = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_dedupe_cluster_report_v1"
    if ($null -eq $dedupe) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            useful_reflection_item_count = $null
            reflection_cluster_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            wasted_compute_guard_count = $null
            pending_operator_approval_count = $null
            adapter_safe_count = $null
            first_cluster_id = $null
            reflection_dedupe_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_dedupe_cluster_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "target_count")
        useful_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "useful_reflection_item_count")
        reflection_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "reflection_cluster_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "duplicate_reflection_item_count")
        wasted_compute_guard_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "wasted_compute_guard_count")
        pending_operator_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "pending_operator_approval_count")
        adapter_safe_count = Convert-ToNullableInt (Get-PropertyValue -Object $dedupe -Name "adapter_safe_count")
        first_cluster_id = Get-PropertyValue -Object $dedupe -Name "first_cluster_id"
        reflection_dedupe_ready = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "reflection_dedupe_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $dedupe -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $dedupe -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReusePlan {
    param([object]$Report)

    $reuse = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_plan_report_v1"
    if ($null -eq $reuse) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            reflection_cluster_count = $null
            plan_item_count = $null
            ready_reuse_plan_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            first_plan_item_id = $null
            reflection_reuse_plan_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_plan_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "target_count")
        reflection_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "reflection_cluster_count")
        plan_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "plan_item_count")
        ready_reuse_plan_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "ready_reuse_plan_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $reuse -Name "projected_saved_reflection_count")
        first_plan_item_id = Get-PropertyValue -Object $reuse -Name "first_plan_item_id"
        reflection_reuse_plan_ready = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "reflection_reuse_plan_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $reuse -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reuse -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReusePreflight {
    param([object]$Report)

    $preflight = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_preflight_report_v1"
    if ($null -eq $preflight) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            plan_item_count = $null
            ready_reuse_plan_count = $null
            preflight_item_count = $null
            preflight_passed_item_count = $null
            blocked_item_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_preflight_item_id = $null
            reuse_preflight_passed = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_preflight_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "target_count")
        plan_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "plan_item_count")
        ready_reuse_plan_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "ready_reuse_plan_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "preflight_item_count")
        preflight_passed_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "preflight_passed_item_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "blocked_item_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $preflight -Name "projected_model_call_skip_count")
        first_preflight_item_id = Get-PropertyValue -Object $preflight -Name "first_preflight_item_id"
        reuse_preflight_passed = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "reuse_preflight_passed")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "reflection_reuse_execution_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $preflight -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $preflight -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupPreview {
    param([object]$Report)

    $lookupPreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1"
    if ($null -eq $lookupPreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            preflight_item_count = $null
            lookup_preview_item_count = $null
            ready_lookup_preview_count = $null
            blocked_item_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_lookup_key = $null
            lookup_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "target_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "preflight_item_count")
        lookup_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "lookup_preview_item_count")
        ready_lookup_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "ready_lookup_preview_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "blocked_item_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $lookupPreview -Name "projected_model_call_skip_count")
        first_lookup_key = Get-PropertyValue -Object $lookupPreview -Name "first_lookup_key"
        lookup_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "lookup_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $lookupPreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $lookupPreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalRequest {
    param([object]$Report)

    $approvalRequest = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1"
    if ($null -eq $approvalRequest) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            preflight_item_count = $null
            lookup_preview_item_count = $null
            ready_lookup_preview_count = $null
            approval_request_item_count = $null
            ready_approval_request_count = $null
            requested_lookup_approval_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_approval_request_id = $null
            lookup_approval_request_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "target_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "preflight_item_count")
        lookup_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "lookup_preview_item_count")
        ready_lookup_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "ready_lookup_preview_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "approval_request_item_count")
        ready_approval_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "ready_approval_request_count")
        requested_lookup_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "requested_lookup_approval_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $approvalRequest -Name "projected_model_call_skip_count")
        first_approval_request_id = Get-PropertyValue -Object $approvalRequest -Name "first_approval_request_id"
        lookup_approval_request_ready = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "lookup_approval_request_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $approvalRequest -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $approvalRequest -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalDecisionPreview {
    param([object]$Report)

    $decisionPreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1"
    if ($null -eq $decisionPreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            preflight_item_count = $null
            lookup_preview_item_count = $null
            ready_lookup_preview_count = $null
            approval_request_item_count = $null
            ready_approval_request_count = $null
            approval_decision_preview_item_count = $null
            ready_approval_decision_preview_count = $null
            approved_lookup_execution_count = $null
            pending_approval_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_approval_decision_preview_id = $null
            lookup_approval_decision_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "target_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "preflight_item_count")
        lookup_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "lookup_preview_item_count")
        ready_lookup_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_lookup_preview_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_request_item_count")
        ready_approval_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_approval_request_count")
        approval_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_decision_preview_item_count")
        ready_approval_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_approval_decision_preview_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approved_lookup_execution_count")
        pending_approval_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "pending_approval_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_model_call_skip_count")
        first_approval_decision_preview_id = Get-PropertyValue -Object $decisionPreview -Name "first_approval_decision_preview_id"
        lookup_approval_decision_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_approval_decision_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $decisionPreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenIntakePreview {
    param([object]$Report)

    $intakePreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1"
    if ($null -eq $intakePreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            preflight_item_count = $null
            lookup_preview_item_count = $null
            ready_lookup_preview_count = $null
            approval_request_item_count = $null
            ready_approval_request_count = $null
            approval_decision_preview_item_count = $null
            ready_approval_decision_preview_count = $null
            token_intake_preview_item_count = $null
            ready_token_intake_preview_count = $null
            pending_operator_token_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_intake_preview_id = $null
            lookup_approval_token_intake_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "target_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "preflight_item_count")
        lookup_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "lookup_preview_item_count")
        ready_lookup_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "ready_lookup_preview_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "approval_request_item_count")
        ready_approval_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "ready_approval_request_count")
        approval_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "approval_decision_preview_item_count")
        ready_approval_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "ready_approval_decision_preview_count")
        token_intake_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "token_intake_preview_item_count")
        ready_token_intake_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "ready_token_intake_preview_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "pending_operator_token_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $intakePreview -Name "projected_model_call_skip_count")
        first_token_intake_preview_id = Get-PropertyValue -Object $intakePreview -Name "first_token_intake_preview_id"
        lookup_approval_token_intake_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "lookup_approval_token_intake_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $intakePreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intakePreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview {
    param([object]$Report)

    $decisionPreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1"
    if ($null -eq $decisionPreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            preflight_item_count = $null
            lookup_preview_item_count = $null
            ready_lookup_preview_count = $null
            approval_request_item_count = $null
            ready_approval_request_count = $null
            approval_decision_preview_item_count = $null
            ready_approval_decision_preview_count = $null
            token_intake_preview_item_count = $null
            ready_token_intake_preview_count = $null
            token_intake_decision_preview_item_count = $null
            ready_token_intake_decision_preview_count = $null
            pending_operator_token_count = $null
            approved_lookup_execution_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_intake_decision_preview_id = $null
            lookup_approval_token_intake_decision_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "target_count")
        preflight_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "preflight_item_count")
        lookup_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "lookup_preview_item_count")
        ready_lookup_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_lookup_preview_count")
        approval_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_request_item_count")
        ready_approval_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_approval_request_count")
        approval_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_decision_preview_item_count")
        ready_approval_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_approval_decision_preview_count")
        token_intake_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "token_intake_preview_item_count")
        ready_token_intake_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_token_intake_preview_count")
        token_intake_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "token_intake_decision_preview_item_count")
        ready_token_intake_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_token_intake_decision_preview_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "pending_operator_token_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approved_lookup_execution_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_model_call_skip_count")
        first_token_intake_decision_preview_id = Get-PropertyValue -Object $decisionPreview -Name "first_token_intake_decision_preview_id"
        lookup_approval_token_intake_decision_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_approval_token_intake_decision_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $decisionPreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview {
    param([object]$Report)

    $recordPreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1"
    if ($null -eq $recordPreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            token_intake_decision_preview_item_count = $null
            ready_token_intake_decision_preview_count = $null
            token_decision_record_preview_item_count = $null
            ready_token_decision_record_preview_count = $null
            pending_operator_token_count = $null
            approved_lookup_execution_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_decision_record_preview_id = $null
            lookup_approval_token_decision_record_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "target_count")
        token_intake_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "token_intake_decision_preview_item_count")
        ready_token_intake_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "ready_token_intake_decision_preview_count")
        token_decision_record_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "token_decision_record_preview_item_count")
        ready_token_decision_record_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "ready_token_decision_record_preview_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "pending_operator_token_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "approved_lookup_execution_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordPreview -Name "projected_model_call_skip_count")
        first_token_decision_record_preview_id = Get-PropertyValue -Object $recordPreview -Name "first_token_decision_record_preview_id"
        lookup_approval_token_decision_record_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "lookup_approval_token_decision_record_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $recordPreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordPreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest {
    param([object]$Report)

    $recordRequest = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1"
    if ($null -eq $recordRequest) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            token_decision_record_preview_item_count = $null
            ready_token_decision_record_preview_count = $null
            token_decision_record_request_item_count = $null
            ready_token_decision_record_request_count = $null
            requested_token_decision_record_count = $null
            pending_operator_token_count = $null
            approved_lookup_execution_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_decision_record_request_id = $null
            lookup_approval_token_decision_record_request_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "target_count")
        token_decision_record_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "token_decision_record_preview_item_count")
        ready_token_decision_record_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "ready_token_decision_record_preview_count")
        token_decision_record_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "token_decision_record_request_item_count")
        ready_token_decision_record_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "ready_token_decision_record_request_count")
        requested_token_decision_record_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "requested_token_decision_record_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "pending_operator_token_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "approved_lookup_execution_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $recordRequest -Name "projected_model_call_skip_count")
        first_token_decision_record_request_id = Get-PropertyValue -Object $recordRequest -Name "first_token_decision_record_request_id"
        lookup_approval_token_decision_record_request_ready = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "lookup_approval_token_decision_record_request_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $recordRequest -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $recordRequest -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket {
    param([object]$Report)

    $reviewPacket = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1"
    if ($null -eq $reviewPacket) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            token_decision_record_request_item_count = $null
            ready_token_decision_record_request_count = $null
            token_decision_record_review_packet_item_count = $null
            ready_token_decision_record_review_packet_count = $null
            requested_token_decision_record_count = $null
            pending_operator_token_count = $null
            approved_lookup_execution_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_decision_record_review_packet_id = $null
            lookup_approval_token_decision_record_review_packet_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "target_count")
        token_decision_record_request_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "token_decision_record_request_item_count")
        ready_token_decision_record_request_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "ready_token_decision_record_request_count")
        token_decision_record_review_packet_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "token_decision_record_review_packet_item_count")
        ready_token_decision_record_review_packet_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "ready_token_decision_record_review_packet_count")
        requested_token_decision_record_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "requested_token_decision_record_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "pending_operator_token_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "approved_lookup_execution_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $reviewPacket -Name "projected_model_call_skip_count")
        first_token_decision_record_review_packet_id = Get-PropertyValue -Object $reviewPacket -Name "first_token_decision_record_review_packet_id"
        lookup_approval_token_decision_record_review_packet_ready = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "lookup_approval_token_decision_record_review_packet_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $reviewPacket -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $reviewPacket -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview {
    param([object]$Report)

    $decisionPreview = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1"
    if ($null -eq $decisionPreview) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            token_decision_record_review_packet_item_count = $null
            ready_token_decision_record_review_packet_count = $null
            token_decision_record_review_packet_decision_preview_item_count = $null
            ready_token_decision_record_review_packet_decision_preview_count = $null
            requested_token_decision_record_count = $null
            pending_operator_token_count = $null
            approved_lookup_execution_count = $null
            blocked_item_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            duplicate_cluster_count = $null
            duplicate_reflection_item_count = $null
            projected_saved_reflection_count = $null
            projected_model_call_skip_count = $null
            first_token_decision_record_review_packet_decision_preview_id = $null
            lookup_approval_token_decision_record_review_packet_decision_preview_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            model_call_skip_authorized = $null
            reflection_reuse_execution_authorized = $null
            memory_lookup_performed = $null
            lookup_hit_assumed = $null
            failure_reasons = @()
            read_only = $null
            report_only = $null
            candidate_only = $null
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "target_count")
        token_decision_record_review_packet_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "token_decision_record_review_packet_item_count")
        ready_token_decision_record_review_packet_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_token_decision_record_review_packet_count")
        token_decision_record_review_packet_decision_preview_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "token_decision_record_review_packet_decision_preview_item_count")
        ready_token_decision_record_review_packet_decision_preview_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "ready_token_decision_record_review_packet_decision_preview_count")
        requested_token_decision_record_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "requested_token_decision_record_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "pending_operator_token_count")
        approved_lookup_execution_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approved_lookup_execution_count")
        blocked_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "blocked_item_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "rejection_token_present_count")
        duplicate_cluster_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_cluster_count")
        duplicate_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "duplicate_reflection_item_count")
        projected_saved_reflection_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_saved_reflection_count")
        projected_model_call_skip_count = Convert-ToNullableInt (Get-PropertyValue -Object $decisionPreview -Name "projected_model_call_skip_count")
        first_token_decision_record_review_packet_decision_preview_id = Get-PropertyValue -Object $decisionPreview -Name "first_token_decision_record_review_packet_decision_preview_id"
        lookup_approval_token_decision_record_review_packet_decision_preview_ready = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_approval_token_decision_record_review_packet_decision_preview_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "admission_write_authorized")
        model_call_skip_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "model_call_skip_authorized")
        reflection_reuse_execution_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "reflection_reuse_execution_authorized")
        memory_lookup_performed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_lookup_performed")
        lookup_hit_assumed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "lookup_hit_assumed")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $decisionPreview -Name "failure_reasons")
        read_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "read_only")
        report_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "report_only")
        candidate_only = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "candidate_only")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $decisionPreview -Name "ndkv_write_allowed")
    }
}

function Get-SelfImproveProposalMemoryApprovalTokenIntakePreview {
    param([object]$Report)

    $intake = Get-PropertyValue -Object $Report -Name "self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1"
    if ($null -eq $intake) {
        return [pscustomobject][ordered]@{
            source = "unavailable"
            target_count = $null
            review_packet_item_count = $null
            useful_reflection_item_count = $null
            intake_item_count = $null
            ready_intake_count = $null
            pending_operator_token_count = $null
            blocked_count = $null
            approval_token_present_count = $null
            rejection_token_present_count = $null
            first_intake_item = $null
            approval_token_intake_ready = $null
            explicit_operator_approval_required = $null
            validation_required = $null
            rollback_required = $null
            commit_allowed = $null
            admission_write_authorized = $null
            failure_reasons = @()
            auto_apply = $null
            memory_store_write_allowed = $null
            ndkv_write_allowed = $null
        }
    }

    return [pscustomobject][ordered]@{
        source = "self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1"
        target_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "target_count")
        review_packet_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "review_packet_item_count")
        useful_reflection_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "useful_reflection_item_count")
        intake_item_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "intake_item_count")
        ready_intake_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "ready_intake_count")
        pending_operator_token_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "pending_operator_token_count")
        blocked_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "blocked_count")
        approval_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "approval_token_present_count")
        rejection_token_present_count = Convert-ToNullableInt (Get-PropertyValue -Object $intake -Name "rejection_token_present_count")
        first_intake_item = Get-PropertyValue -Object $intake -Name "first_intake_item_id"
        approval_token_intake_ready = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "approval_token_intake_ready")
        explicit_operator_approval_required = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "explicit_operator_approval_required")
        validation_required = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "validation_required")
        rollback_required = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "rollback_required")
        commit_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "commit_allowed")
        admission_write_authorized = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "admission_write_authorized")
        failure_reasons = Convert-ToStringList (Get-PropertyValue -Object $intake -Name "failure_reasons")
        auto_apply = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "auto_apply")
        memory_store_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "memory_store_write_allowed")
        ndkv_write_allowed = Convert-ToNullableBool (Get-PropertyValue -Object $intake -Name "ndkv_write_allowed")
    }
}

function Select-FirstNonNull {
    param(
        [object]$Preferred,
        [object]$Fallback
    )

    if ($null -ne $Preferred) {
        return $Preferred
    }
    return $Fallback
}

function Select-FirstNonEmptyString {
    param(
        [object]$Preferred,
        [object]$Fallback
    )

    if ($null -ne $Preferred -and -not [string]::IsNullOrWhiteSpace([string]$Preferred)) {
        return $Preferred
    }
    return $Fallback
}

function Select-FirstNonEmptyList {
    param(
        [object]$Preferred,
        [object]$Fallback
    )

    $preferredItems = @(Convert-ToStringList $Preferred)
    if ($preferredItems.Count -gt 0) {
        return $preferredItems
    }
    return @(Convert-ToStringList $Fallback)
}

function Resolve-SelfImproveProposalActionPlan {
    param(
        [object]$ActionPlan,
        [object]$ConvertAdvisory,
        [object]$RepairUnvalidated,
        [object]$RequiresValidation,
        [bool]$Available
    )

    if (-not $Available) {
        return [pscustomobject][ordered]@{
            action_required = $null
            primary_action = $null
            actions = @()
            requires_checked_passed_validation_and_accepted_memory_admission = $null
        }
    }

    if ($null -ne $ActionPlan) {
        $actions = Convert-ToStringList (Get-PropertyValue -Object $ActionPlan -Name "actions")
        $primary = Get-PropertyValue -Object $ActionPlan -Name "primary_action"
        if ($null -eq $primary -or [string]::IsNullOrWhiteSpace([string]$primary)) {
            $primary = if ($actions.Count -gt 0) { [string]$actions[0] } else { "none" }
        }
        $actionRequired = Convert-ToNullableBool (Get-PropertyValue -Object $ActionPlan -Name "action_required")
        if ($null -eq $actionRequired) {
            $actionRequired = [bool]($actions.Count -gt 0)
        }
        $requires = Convert-ToNullableBool (Get-PropertyValue -Object $ActionPlan -Name "requires_checked_passed_validation_and_accepted_memory_admission")
        if ($null -eq $requires) {
            $requires = if ($RequiresValidation -eq $true) { $true } else { $false }
        }
        return [pscustomobject][ordered]@{
            action_required = $actionRequired
            primary_action = [string]$primary
            actions = $actions
            requires_checked_passed_validation_and_accepted_memory_admission = $requires
        }
    }

    $derivedActions = @()
    if ($ConvertAdvisory -eq $true) {
        $derivedActions += "convert_advisory_to_evidence_backed_business_improvement"
    }
    if ($RepairUnvalidated -eq $true) {
        $derivedActions += "repair_unvalidated_or_unaccepted_proposals"
    }
    if ($RequiresValidation -eq $true) {
        $derivedActions += "require_checked_passed_validation_and_accepted_memory_admission"
    }
    return [pscustomobject][ordered]@{
        action_required = [bool]($derivedActions.Count -gt 0)
        primary_action = if ($derivedActions.Count -gt 0) { [string]$derivedActions[0] } else { "none" }
        actions = $derivedActions
        requires_checked_passed_validation_and_accepted_memory_admission = if ($RequiresValidation -eq $true) { $true } else { $false }
    }
}

function Convert-ToStringList {
    param([object]$Value)

    if ($null -eq $Value) {
        return @()
    }
    $items = @()
    foreach ($item in @($Value)) {
        if ($null -ne $item -and -not [string]::IsNullOrWhiteSpace([string]$item)) {
            $items += [string]$item
        }
    }
    return $items
}

function Resolve-SelfImproveProposalGuidanceBool {
    param(
        [object]$Value,
        [object]$Business,
        [object]$Advisory,
        [object]$Repair,
        [object]$AcceptedWithoutEvidence,
        [string]$Name
    )

    if ($null -ne $Value) {
        return [bool]$Value
    }
    $businessCount = if ($null -eq $Business) { 0 } else { [int]$Business }
    $advisoryCount = if ($null -eq $Advisory) { 0 } else { [int]$Advisory }
    $repairCount = if ($null -eq $Repair) { 0 } else { [int]$Repair }
    $acceptedWithoutEvidenceCount = if ($null -eq $AcceptedWithoutEvidence) { 0 } else { [int]$AcceptedWithoutEvidence }
    if ($Name -eq "convert") {
        return [bool]($businessCount -eq 0 -and $advisoryCount -gt 0)
    }
    if ($Name -eq "repair") {
        return [bool]($repairCount -gt 0 -or $acceptedWithoutEvidenceCount -gt 0)
    }
    if ($Name -eq "requires_validation") {
        return [bool](($businessCount + $advisoryCount + $repairCount + $acceptedWithoutEvidenceCount) -gt 0)
    }
    return $null
}

function Normalize-BackendBaseUri {
    param([string]$Backend)

    if ([string]::IsNullOrWhiteSpace($Backend)) {
        return ""
    }
    $base = $Backend.Trim()
    if (-not ($base.StartsWith("http://") -or $base.StartsWith("https://"))) {
        $base = "http://$base"
    }
    return $base.TrimEnd("/")
}

function Record-Number {
    param(
        [object]$Record,
        [string]$Name,
        [string]$NestedName = ""
    )

    if (Has-Property $Record $Name) {
        return [double]$Record.$Name
    }
    if ($NestedName.Trim().Length -gt 0 -and (Has-Property $Record "business_cycle")) {
        $cycle = $Record.business_cycle
        if (Has-Property $cycle $NestedName) {
            return [double]$cycle.$NestedName
        }
    }
    return 0.0
}

function Record-Bool {
    param(
        [object]$Record,
        [string]$Name,
        [string]$NestedName = ""
    )

    if (Has-Property $Record $Name) {
        return [bool]$Record.$Name
    }
    if ($NestedName.Trim().Length -gt 0 -and (Has-Property $Record "business_cycle")) {
        $cycle = $Record.business_cycle
        if (Has-Property $cycle $NestedName) {
            return [bool]$cycle.$NestedName
        }
    }
    return $false
}

function Sum-RecordNumber {
    param(
        [object[]]$Records,
        [string]$Name,
        [string]$NestedName = ""
    )

    $total = 0.0
    foreach ($record in $Records) {
        $total += Record-Number -Record $record -Name $Name -NestedName $NestedName
    }
    return $total
}

function Parse-CommaList {
    param([string]$Value)

    $items = @()
    foreach ($item in $Value.Split(",")) {
        $trimmed = $item.Trim()
        if ($trimmed.Length -gt 0 -and $items -notcontains $trimmed) {
            $items += $trimmed
        }
    }
    return $items
}

function HelperStage-Roles {
    param([object]$Record)

    if (-not (Has-Property $Record "helper_stage_feedback_by_role")) {
        return @()
    }
    $roles = @()
    foreach ($property in @($Record.helper_stage_feedback_by_role.PSObject.Properties)) {
        $items = @($property.Value | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
        if ($items.Count -gt 0) {
            $roles += [string]$property.Name
        }
    }
    return @($roles | Sort-Object)
}

function HelperStage-FeedbackTotal {
    param([object]$Record)

    if (-not (Has-Property $Record "helper_stage_feedback_by_role")) {
        return 0
    }
    $total = 0
    foreach ($property in @($Record.helper_stage_feedback_by_role.PSObject.Properties)) {
        $total += @($property.Value | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count
    }
    return $total
}

function HelperStage-ContractRoles {
    param([object]$Record)

    if (-not (Has-Property $Record "helper_stage_contract_by_role")) {
        return @()
    }
    return @($Record.helper_stage_contract_by_role.PSObject.Properties | ForEach-Object { [string]$_.Name } | Sort-Object)
}

function HelperStage-MarkerList {
    param(
        [object]$Entry,
        [string]$Name
    )

    if (-not (Has-Property $Entry $Name)) {
        return @()
    }
    return @($Entry.$Name | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function HelperStage-FieldNames {
    param([object]$Entry)

    if (-not (Has-Property $Entry "fields")) {
        return @()
    }
    return @($Entry.fields.PSObject.Properties | ForEach-Object { [string]$_.Name } | Sort-Object)
}

function HelperStage-ContractSummary {
    param(
        [object]$Record,
        [string[]]$RequiredRoles
    )

    $roles = @($RequiredRoles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($roles.Count -eq 0) {
        $roles = HelperStage-ContractRoles -Record $Record
    }
    $completeRoles = @()
    $incompleteRoles = @()
    if (-not (Has-Property $Record "helper_stage_contract_by_role")) {
        foreach ($role in $roles) {
            $incompleteRoles += $role
        }
        return [pscustomobject][ordered]@{
            complete = $false
            checked_roles = @($roles)
            complete_roles = @()
            incomplete_roles = @($incompleteRoles | Sort-Object)
        }
    }

    $contracts = $Record.helper_stage_contract_by_role
    foreach ($role in $roles) {
        $entry = Get-PropertyValue -Object $contracts -Name $role
        if ($null -eq $entry) {
            $incompleteRoles += $role
            continue
        }
        $expected = HelperStage-MarkerList -Entry $entry -Name "expected_markers"
        $matched = HelperStage-MarkerList -Entry $entry -Name "matched_markers"
        if ($expected.Count -eq 0 -and $matched.Count -eq 0) {
            $expected = HelperStage-FieldNames -Entry $entry
            $matched = @($expected)
        }
        $missingMarkers = @()
        foreach ($marker in $expected) {
            if ($matched -notcontains $marker) {
                $missingMarkers += $marker
            }
        }
        if ($expected.Count -gt 0 -and $missingMarkers.Count -eq 0) {
            $completeRoles += $role
        } else {
            $incompleteRoles += $role
        }
    }

    return [pscustomobject][ordered]@{
        complete = $roles.Count -gt 0 -and $incompleteRoles.Count -eq 0
        checked_roles = @($roles | Sort-Object)
        complete_roles = @($completeRoles | Sort-Object)
        incomplete_roles = @($incompleteRoles | Sort-Object)
    }
}

function HelperStage-ContractField {
    param(
        [object]$Record,
        [string]$Role,
        [string]$FieldName
    )

    if (-not (Has-Property $Record "helper_stage_contract_by_role")) {
        return ""
    }
    $entry = Get-PropertyValue -Object $Record.helper_stage_contract_by_role -Name $Role
    if ($null -eq $entry -or -not (Has-Property $entry "fields")) {
        return ""
    }
    $value = Get-PropertyValue -Object $entry.fields -Name $FieldName
    if ($null -eq $value) {
        return ""
    }
    return [string]$value
}

function TestGate-ValidationCommandSafety {
    param([string]$Command)

    $trimmed = $Command.Trim()
    if ($trimmed.Length -eq 0) {
        return "missing"
    }
    if ($trimmed.Length -gt 240) {
        return "unsafe"
    }
    $lower = $trimmed.ToLowerInvariant()
    if ($lower -match '[;&|<>`]' -or $lower -match '\b(remove-item|rm|del|erase|rmdir|rd)\b') {
        return "unsafe"
    }
    if ($lower -match '\s--fix\b') {
        return "unsafe"
    }
    if ($lower -match '^cargo\s+(test|check|clippy)\b') {
        return "safe"
    }
    if ($lower -match '^cargo\s+fmt\b' -and $lower -match '(^|\s)--check(\s|$)') {
        return "safe"
    }
    return "unsafe"
}

function Read-Ledger {
    param([string]$Path)

    $records = @()
    $invalid = 0
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject]@{
            exists = $false
            records = @()
            invalid_records = 0
        }
    }

    foreach ($line in Get-Content -LiteralPath $Path) {
        if ($line.Trim().Length -eq 0) {
            continue
        }
        try {
            $records += ($line | ConvertFrom-Json)
        } catch {
            $invalid += 1
        }
    }
    return [pscustomobject]@{
        exists = $true
        records = $records
        invalid_records = $invalid
    }
}

function Ledger-Summary {
    param(
        [string]$Path,
        [object[]]$Records,
        [int]$InvalidRecords
    )

    $rounds = @()
    foreach ($record in $Records) {
        if (Has-Property $record "round") {
            $rounds += [int]$record.round
        }
    }

    $uniqueRounds = @($rounds | Sort-Object -Unique)
    $duplicateRounds = [Math]::Max(0, $rounds.Count - $uniqueRounds.Count)
    $nonMonotonicRounds = 0
    $previousRound = $null
    foreach ($round in $rounds) {
        if ($null -ne $previousRound -and $round -le $previousRound) {
            $nonMonotonicRounds += 1
        }
        $previousRound = $round
    }

    $roundGaps = 0
    for ($i = 1; $i -lt $uniqueRounds.Count; $i += 1) {
        $gap = [int]$uniqueRounds[$i] - [int]$uniqueRounds[$i - 1] - 1
        if ($gap -gt 0) {
            $roundGaps += $gap
        }
    }

    $successes = @($Records | Where-Object { Has-Property $_ "success" -and $_.success -eq $true }).Count
    $successRate = if ($Records.Count -gt 0) { [Math]::Round(($successes * 100.0) / $Records.Count, 2) } else { 0.0 }
    $latest = if ($Records.Count -gt 0) { $Records[$Records.Count - 1] } else { $null }
    $feedbackTotal = Sum-RecordNumber -Records $Records -Name "feedback_applied" -NestedName "feedback_applied"
    $runtimeTokensTotal = Sum-RecordNumber -Records $Records -Name "runtime_tokens" -NestedName "runtime_token_count"

    $latestSummary = $null
    if ($null -ne $latest) {
        $helperStageRoles = HelperStage-Roles -Record $latest
        $helperStageContractRoles = HelperStage-ContractRoles -Record $latest
        $helperStageContractSummary = HelperStage-ContractSummary `
            -Record $latest `
            -RequiredRoles (Parse-CommaList -Value $RequireLatestHelperStageRolesEffective)
        $testGateVerdict = HelperStage-ContractField -Record $latest -Role "test-gate" -FieldName "verdict"
        $testGateValidationCommand = HelperStage-ContractField -Record $latest -Role "test-gate" -FieldName "validation_command"
        $testGateValidationCommandSafety = TestGate-ValidationCommandSafety -Command $testGateValidationCommand
        $latestSummary = [pscustomobject][ordered]@{
            round = if (Has-Property $latest "round") { [int]$latest.round } else { $null }
            case = if (Has-Property $latest "case") { [string]$latest.case } else { "" }
            success = if (Has-Property $latest "success") { [bool]$latest.success } else { $false }
            error = if (Has-Property $latest "error") { [string]$latest.error } else { "" }
            runtime_tokens = [int](Record-Number -Record $latest -Name "runtime_tokens" -NestedName "runtime_token_count")
            elapsed_ms = [int](Record-Number -Record $latest -Name "elapsed_ms" -NestedName "elapsed_ms")
            feedback_applied = [int](Record-Number -Record $latest -Name "feedback_applied" -NestedName "feedback_applied")
            self_improve_passed = Record-Bool -Record $latest -Name "self_improve_passed" -NestedName "self_improve_passed"
            validation_checked = Record-Bool -Record $latest -Name "validation_checked" -NestedName "validation_checked"
            validation_passed = Record-Bool -Record $latest -Name "validation_passed" -NestedName "validation_passed"
            validation_command_source = if (Has-Property $latest "validation_command_source") { [string]$latest.validation_command_source } else { "" }
            validation_command_safety = if (Has-Property $latest "validation_command_safety") { [string]$latest.validation_command_safety } else { "" }
            validation_status_code = if ((Has-Property $latest "validation_status_code") -and $null -ne $latest.validation_status_code) { [int]$latest.validation_status_code } else { $null }
            validation_elapsed_ms = if ((Has-Property $latest "validation_elapsed_ms") -and $null -ne $latest.validation_elapsed_ms) { [int]$latest.validation_elapsed_ms } else { $null }
            helper_stage_roles = @($helperStageRoles)
            helper_stage_role_count = @($helperStageRoles).Count
            helper_stage_feedback_total = HelperStage-FeedbackTotal -Record $latest
            helper_stage_contract_roles = @($helperStageContractRoles)
            helper_stage_contract_complete = [bool]$helperStageContractSummary.complete
            helper_stage_contract_checked_roles = @($helperStageContractSummary.checked_roles)
            helper_stage_contract_complete_roles = @($helperStageContractSummary.complete_roles)
            helper_stage_contract_incomplete_roles = @($helperStageContractSummary.incomplete_roles)
            test_gate_verdict = $testGateVerdict
            test_gate_passed = $testGateVerdict.Trim().ToLowerInvariant() -eq "pass"
            test_gate_validation_command = $testGateValidationCommand
            test_gate_validation_command_safety = $testGateValidationCommandSafety
        }
    }

    return [pscustomobject][ordered]@{
        path = $Path
        exists = (Test-Path -LiteralPath $Path -PathType Leaf)
        total_records = $Records.Count
        invalid_records = $InvalidRecords
        unique_rounds = $uniqueRounds.Count
        duplicate_rounds = $duplicateRounds
        non_monotonic_rounds = $nonMonotonicRounds
        round_gaps = $roundGaps
        success_count = $successes
        success_rate = $successRate
        feedback_applied_total = [int]$feedbackTotal
        runtime_tokens_total = [int]$runtimeTokensTotal
        latest = $latestSummary
    }
}

function Test-WorkerRuntimeAccelerated {
    param([object]$Worker)

    if ($null -eq $Worker) {
        return $false
    }
    $accelerator = [string](Get-PropertyValue -Object $Worker -Name "runtime_accelerator")
    $device = [string](Get-PropertyValue -Object $Worker -Name "runtime_device")
    $gpuLayers = Get-PropertyValue -Object $Worker -Name "gpu_layers"
    $runtimeText = "$accelerator $device".ToLowerInvariant()
    if ($runtimeText -match "metal|cuda|vulkan|gpu|directml") {
        return $true
    }
    try {
        return [int]$gpuLayers -gt 0
    } catch {
        return $false
    }
}

function Get-QualityWorker {
    param([object]$Status)

    $workers = Get-PropertyValue -Object $Status -Name "workers"
    if ($null -eq $workers -and (Has-Property $Status "model_pool")) {
        $workers = Get-PropertyValue -Object $Status.model_pool -Name "workers"
    }
    foreach ($worker in @($workers | Where-Object { $_ })) {
        if ([string](Get-PropertyValue -Object $worker -Name "role") -eq "quality") {
            return $worker
        }
    }
    return $null
}

function Read-BackendModelPoolHealthFallback {
    param(
        [string]$Backend,
        [string]$HealthError
    )

    $base = Normalize-BackendBaseUri -Backend $Backend
    if ($base.Trim().Length -eq 0) {
        return $null
    }
    try {
        $status = Invoke-RestMethod -Uri "$base/v1/model-pool/status" -TimeoutSec 5
        $quality = Get-QualityWorker -Status $status
        $capacity = Get-PropertyValue -Object $status -Name "capacity"
        $capacityQualityAccelerated = Get-PropertyValue -Object $capacity -Name "quality_runtime_accelerated"
        $qualityAccelerated = if ($null -ne $capacityQualityAccelerated) {
            [bool]$capacityQualityAccelerated
        } else {
            Test-WorkerRuntimeAccelerated -Worker $quality
        }
        $launchAllowed = if (Has-Property $status "launch_allowed") { [bool]$status.launch_allowed } else { $false }
        $qualityReady = if (Has-Property $status "quality_ready") { [bool]$status.quality_ready } else { $null -ne $quality }
        $statusOk = if (Has-Property $status "ok") { [bool]$status.ok } else { $true }
        $healthyWorkers = Convert-ToPositiveInt (Get-PropertyValue -Object $status -Name "healthy_worker_count")
        $routeMetrics = Get-PropertyValue -Object $status -Name "route_metrics"
        $routeInFlight = Convert-ToPositiveInt (Get-PropertyValue -Object $routeMetrics -Name "in_flight")
        $modelName = if ($null -ne $quality) { [string](Get-PropertyValue -Object $quality -Name "model") } else { "" }
        $readinessOk = $statusOk -and $launchAllowed -and $qualityReady -and ($healthyWorkers -gt 0)
        $healthDegraded = -not [string]::IsNullOrWhiteSpace($HealthError)
        return [pscustomobject][ordered]@{
            checked = $true
            ok = $readinessOk
            readiness_ok = $readinessOk
            safe_device_ok = $qualityAccelerated
            engine_busy = $routeInFlight -gt 0
            active_engine_requests = $routeInFlight
            gemma_runtime_reachable = $readinessOk
            gemma_runtime_model = $modelName
            source = "model_pool_status"
            source_detail = "model_pool_status_after_health_error"
            health_fallback_used = $true
            health_degraded = $healthDegraded
            health_error = $HealthError
            error = ""
        }
    } catch {
        return $null
    }
}

function Read-BackendHealth {
    param([string]$Backend)

    $backendHealthFixtureJson = $BackendHealthJson
    if ($BackendHealthJsonPath.Trim().Length -gt 0) {
        $backendHealthFixtureJson = Get-Content -LiteralPath (Resolve-RepoPath $BackendHealthJsonPath) -Raw
    }
    if ($backendHealthFixtureJson.Trim().Length -gt 0) {
        $health = $backendHealthFixtureJson | ConvertFrom-Json
        return [pscustomobject][ordered]@{
            checked = $true
            ok = [bool]$health.ok
            readiness_ok = [bool]$health.readiness_ok
            safe_device_ok = [bool]$health.safe_device_ok
            engine_busy = [bool]$health.engine_busy
            active_engine_requests = [int]$health.active_engine_requests
            gemma_runtime_reachable = [bool]$health.gemma_runtime_reachable
            gemma_runtime_model = [string]$health.gemma_runtime_model
            source = "fixture"
            source_detail = "backend_health_json"
            health_fallback_used = $false
            health_degraded = $false
            health_error = ""
            error = ""
        }
    }

    if ($SkipBackend) {
        return [pscustomobject][ordered]@{
            checked = $false
            ok = $null
            error = ""
        }
    }

    $base = Normalize-BackendBaseUri -Backend $Backend
    try {
        $health = Invoke-RestMethod -Uri "$base/health" -TimeoutSec 5
        return [pscustomobject][ordered]@{
            checked = $true
            ok = [bool]$health.ok
            readiness_ok = [bool]$health.readiness_ok
            safe_device_ok = [bool]$health.safe_device_ok
            engine_busy = [bool]$health.engine_busy
            active_engine_requests = [int]$health.active_engine_requests
            gemma_runtime_reachable = [bool]$health.gemma_runtime_reachable
            gemma_runtime_model = [string]$health.gemma_runtime_model
            source = "health"
            source_detail = "health"
            health_fallback_used = $false
            health_degraded = $false
            health_error = ""
            error = ""
        }
    } catch {
        $fallback = Read-BackendModelPoolHealthFallback -Backend $Backend -HealthError $_.Exception.Message
        if ($null -ne $fallback) {
            return $fallback
        }
        return [pscustomobject][ordered]@{
            checked = $true
            ok = $false
            source = "health"
            source_detail = "health_error_no_fallback"
            health_fallback_used = $false
            health_degraded = $true
            health_error = $_.Exception.Message
            error = $_.Exception.Message
        }
    }
}

function Test-BackendBusyDuringActiveDaemon {
    param(
        [object]$BackendHealth,
        [object]$DaemonStatus
    )

    if (
        -not $BackendHealth.checked `
        -or -not $DaemonStatus.checked `
        -or -not $DaemonStatus.running `
        -or -not $DaemonStatus.activity_ok `
        -or ([string]$DaemonStatus.activity_state -ne "active" -and [string]$DaemonStatus.activity_state -ne "slow_in_progress")
    ) {
        return $false
    }

    $activeEngineRequests = if (Has-Property $BackendHealth "active_engine_requests") {
        Convert-ToPositiveInt $BackendHealth.active_engine_requests
    } else {
        0
    }
    $engineBusy = (Has-Property $BackendHealth "engine_busy") -and [bool]$BackendHealth.engine_busy
    if (-not $engineBusy -and $activeEngineRequests -le 0) {
        return $false
    }

    return (
        [bool]$BackendHealth.ok `
        -and [bool]$BackendHealth.safe_device_ok `
        -and -not [bool]$BackendHealth.readiness_ok
    )
}

function Read-RemoteChainStatus {
    param([string]$Path)

    if ($SkipRemoteChain -or $Path.Trim().Length -eq 0) {
        return [pscustomobject][ordered]@{
            checked = $false
            exists = $false
            ready = $null
            error = ""
        }
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject][ordered]@{
            checked = $true
            exists = $false
            ready = $false
            error = "remote chain status file missing"
        }
    }
    try {
        $status = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $modelCache = if (Has-Property $status "model_cache") { $status.model_cache } else { $null }
        $remoteRuntime = if (Has-Property $status "remote_runtime") { $status.remote_runtime } else { $null }
        $remoteRuntimeSummary = if ($null -eq $remoteRuntime) {
            $null
        } else {
            $remoteRuntimeWorkers = if (Has-Property $remoteRuntime "workers") {
                @($remoteRuntime.workers | Where-Object { $_ })
            } else {
                @()
            }
            $cpuOrNoGpuRoles = @($remoteRuntimeWorkers | Where-Object {
                (Has-Property $_ "cpu_or_no_gpu") -and $_.cpu_or_no_gpu -eq $true
            } | ForEach-Object { [string]$_.role } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
            $metadataMayDifferRoles = @($remoteRuntimeWorkers | Where-Object {
                (Has-Property $_ "backend_metadata_may_differ") -and $_.backend_metadata_may_differ -eq $true
            } | ForEach-Object { [string]$_.role } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
            $capacity = if (Has-Property $status "model_pool") { Get-PropertyValue -Object $status.model_pool -Name "capacity" } else { $null }
            $capacityQualityAccelerated = Get-PropertyValue -Object $capacity -Name "quality_runtime_accelerated"
            $qualityAccelerated = if ($null -ne $capacityQualityAccelerated) {
                [bool]$capacityQualityAccelerated
            } else {
                Test-WorkerRuntimeAccelerated -Worker (Get-QualityWorker -Status $status)
            }
            $qualityCpuOrNoGpuRoles = @($cpuOrNoGpuRoles | Where-Object { $_.Trim().ToLowerInvariant() -eq "quality" })
            $runtimeProbed = if (Has-Property $remoteRuntime "probed") { [bool]$remoteRuntime.probed } else { $null }
            $accelerationOk = ($runtimeProbed -eq $true) -and $qualityAccelerated -and ($cpuOrNoGpuRoles.Count -eq 0) -and ($qualityCpuOrNoGpuRoles.Count -eq 0)
            $accelerationNextStep = if ($accelerationOk) {
                ""
            } else {
                ".\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
            }
            [pscustomobject][ordered]@{
                probed = $runtimeProbed
                touches_remote = if (Has-Property $remoteRuntime "touches_remote") { [bool]$remoteRuntime.touches_remote } else { $null }
                worker_count = if (Has-Property $remoteRuntime "worker_count") { [int]$remoteRuntime.worker_count } else { $null }
                cpu_or_no_gpu_count = if (Has-Property $remoteRuntime "cpu_or_no_gpu_count") { [int]$remoteRuntime.cpu_or_no_gpu_count } else { $null }
                cpu_or_no_gpu_roles = $cpuOrNoGpuRoles
                backend_metadata_may_differ_roles = $metadataMayDifferRoles
                acceleration_ok = $accelerationOk
                acceleration_next_step = $accelerationNextStep
                error = if (Has-Property $remoteRuntime "error") { [string]$remoteRuntime.error } else { "" }
            }
        }
        return [pscustomobject][ordered]@{
            checked = $true
            exists = $true
            path = $Path
            ready = [bool]$status.readiness.ready
            model_api = [bool]$status.readiness.model_api
            backend = [bool]$status.readiness.backend
            web_lab = [bool]$status.readiness.web_lab
            quality_worker = [bool]$status.readiness.quality_worker
            worker_count = [int]$status.model_pool.worker_count
            healthy_worker_count = [int]$status.model_pool.healthy_worker_count
            model_cache_all_ok = if ($null -eq $modelCache) { $null } else { [bool]$modelCache.all_ok }
            model_cache_ok_count = if ($null -eq $modelCache) { 0 } else { [int]$modelCache.ok_count }
            model_cache_model_count = if ($null -eq $modelCache) { 0 } else { [int]$modelCache.model_count }
            model_cache_remote_error_count = if ($null -eq $modelCache) { 0 } else { [int]$modelCache.remote_error_count }
            model_cache_path = if ($null -eq $modelCache) { "" } else { [string]$modelCache.path }
            remote_runtime = $remoteRuntimeSummary
            error = ""
        }
    } catch {
        return [pscustomobject][ordered]@{
            checked = $true
            exists = $true
            ready = $false
            error = $_.Exception.Message
        }
    }
}

function Read-ReportStatus {
    param(
        [string]$Path,
        [object]$LedgerSummary
    )

    $empty = [pscustomobject][ordered]@{
        path = $Path
        exists = $false
        parse_error = ""
        rounds = $null
        success = $null
        failures = $null
        success_rate = $null
        report_gate_passed = $null
        report_gate_failure_count = $null
        self_improve_proposal_acceptance_summary_source = "unavailable"
        self_improve_proposal_business_count = $null
        self_improve_proposal_advisory_count = $null
        self_improve_proposal_repair_count = $null
        self_improve_proposal_accepted_without_business_evidence_count = $null
        self_improve_proposal_convert_advisory_to_business_evidence = $null
        self_improve_proposal_repair_unvalidated_or_unaccepted = $null
        self_improve_proposal_requires_validation_and_memory_admission = $null
        self_improve_proposal_action_required = $null
        self_improve_proposal_primary_action = $null
        self_improve_proposal_actions = @()
        self_improve_proposal_action_plan_requires_validation_and_memory_admission = $null
        self_improve_proposal_action_assignment_source = "unavailable"
        self_improve_proposal_action_assignment_target_count = $null
        self_improve_proposal_action_assignment_first_target = $null
        self_improve_proposal_action_assignment_first_missing_requirements = @()
        ledger_lag_rounds = $null
        stale = $null
        remote_runtime_probed = $null
        remote_runtime_acceleration_ok = $null
    }
    if ($Path.Trim().Length -eq 0) {
        return $empty
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject][ordered]@{
            path = $Path
            exists = $false
            parse_error = ""
            rounds = $null
            success = $null
            failures = $null
            success_rate = $null
            report_gate_passed = $null
            report_gate_failure_count = $null
            self_improve_proposal_acceptance_summary_source = "unavailable"
            self_improve_proposal_business_count = $null
            self_improve_proposal_advisory_count = $null
            self_improve_proposal_repair_count = $null
            self_improve_proposal_accepted_without_business_evidence_count = $null
            self_improve_proposal_convert_advisory_to_business_evidence = $null
            self_improve_proposal_repair_unvalidated_or_unaccepted = $null
            self_improve_proposal_requires_validation_and_memory_admission = $null
            self_improve_proposal_action_required = $null
            self_improve_proposal_primary_action = $null
            self_improve_proposal_actions = @()
            self_improve_proposal_action_plan_requires_validation_and_memory_admission = $null
            self_improve_proposal_action_assignment_source = "unavailable"
            self_improve_proposal_action_assignment_target_count = $null
            self_improve_proposal_action_assignment_first_target = $null
            self_improve_proposal_action_assignment_first_missing_requirements = @()
            ledger_lag_rounds = $null
            stale = $null
            remote_runtime_probed = $null
            remote_runtime_acceleration_ok = $null
        }
    }

    try {
        $report = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $rounds = Convert-ToNullableInt (Get-PropertyValue -Object $report -Name "rounds")
        $ledgerRecords = Convert-ToNullableInt (Get-PropertyValue -Object $LedgerSummary -Name "total_records")
        $lag = if ($null -ne $rounds -and $null -ne $ledgerRecords) { [int]($ledgerRecords - $rounds) } else { $null }
        $reportGate = Get-PropertyValue -Object $report -Name "report_gate"
        $gateFailures = if ($null -ne $reportGate -and (Has-Property $reportGate "failures")) {
            @($reportGate.failures | Where-Object { $null -ne $_ })
        } else {
            @()
        }
        $remoteChain = Get-PropertyValue -Object $report -Name "remote_chain"
        $remoteRuntime = Get-PropertyValue -Object $remoteChain -Name "remote_runtime"
        $proposalAcceptanceSummary = Get-SelfImproveProposalAcceptanceSummary -Report $report
        $proposalRepairFactorReadiness = Get-SelfImproveProposalRepairFactorReadiness -Report $report
        $proposalActionClosure = Get-SelfImproveProposalActionClosure -Report $report
        $proposalMemoryAdmissionReadiness = Get-SelfImproveProposalMemoryAdmissionReadiness -Report $report
        $proposalMemoryAdmissionRequest = Get-SelfImproveProposalMemoryAdmissionRequest -Report $report
        $proposalMemoryAdmissionDecision = Get-SelfImproveProposalMemoryAdmissionDecision -Report $report
        $proposalMemoryAdmissionWriterPlan = Get-SelfImproveProposalMemoryAdmissionWriterPlan -Report $report
        $proposalMemoryAdmissionWriterDryRun = Get-SelfImproveProposalMemoryAdmissionWriterDryRun -Report $report
        $proposalMemoryAdmissionWriterDryRunReceipt = Get-SelfImproveProposalMemoryAdmissionWriterDryRunReceipt -Report $report
        $proposalMemoryAdmissionCommitRecordStage = Get-SelfImproveProposalMemoryAdmissionCommitRecordStage -Report $report
        $proposalMemoryAdmissionCommitApprovalRequest = Get-SelfImproveProposalMemoryAdmissionCommitApprovalRequest -Report $report
        $proposalMemoryAdmissionCommitApprovalDecision = Get-SelfImproveProposalMemoryAdmissionCommitApprovalDecision -Report $report
        $proposalMemoryAdmissionCommitApprovalReviewPacket = Get-SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacket -Report $report
        $proposalMemoryReflectionUsefulness = Get-SelfImproveProposalMemoryReflectionUsefulness -Report $report
        $proposalMemoryReflectionDedupeCluster = Get-SelfImproveProposalMemoryReflectionDedupeCluster -Report $report
        $proposalMemoryReflectionReusePlan = Get-SelfImproveProposalMemoryReflectionReusePlan -Report $report
        $proposalMemoryReflectionReusePreflight = Get-SelfImproveProposalMemoryReflectionReusePreflight -Report $report
        $proposalMemoryReflectionReuseLookupPreview = Get-SelfImproveProposalMemoryReflectionReuseLookupPreview -Report $report
        $proposalMemoryReflectionReuseLookupApprovalRequest = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalRequest -Report $report
        $proposalMemoryReflectionReuseLookupApprovalDecisionPreview = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalDecisionPreview -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Report $report
        $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview = Get-SelfImproveProposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Report $report
        $proposalMemoryApprovalTokenIntakePreview = Get-SelfImproveProposalMemoryApprovalTokenIntakePreview -Report $report
        $proposalRepairFactorRelease = Get-SelfImproveProposalRepairFactorRelease -Report $report
        $proposalRepairFactorRetagPlan = Get-SelfImproveProposalRepairFactorRetagPlan -Report $report

        return [pscustomobject][ordered]@{
            path = $Path
            exists = $true
            parse_error = ""
            rounds = $rounds
            success = Convert-ToNullableInt (Get-PropertyValue -Object $report -Name "success")
            failures = Convert-ToNullableInt (Get-PropertyValue -Object $report -Name "failures")
            success_rate = Convert-ToNullableDouble (Get-PropertyValue -Object $report -Name "success_rate")
            report_gate_passed = Convert-ToNullableBool (Get-PropertyValue -Object $reportGate -Name "passed")
            report_gate_failure_count = if ($null -ne $reportGate) { [int]$gateFailures.Count } else { $null }
            self_improve_proposal_acceptance_summary_source = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "source"
            self_improve_proposal_business_count = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "evidence_backed_business_improvement_count"
            self_improve_proposal_advisory_count = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "advisory_only_count"
            self_improve_proposal_repair_count = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "require_repair_count"
            self_improve_proposal_accepted_without_business_evidence_count = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "accepted_without_business_evidence_count"
            self_improve_proposal_convert_advisory_to_business_evidence = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "should_convert_advisory_to_evidence_backed_business_improvement"
            self_improve_proposal_repair_unvalidated_or_unaccepted = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "should_repair_unvalidated_or_unaccepted_proposals"
            self_improve_proposal_requires_validation_and_memory_admission = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "requires_checked_passed_validation_and_accepted_memory_admission"
            self_improve_proposal_action_required = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_required"
            self_improve_proposal_primary_action = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "primary_action"
            self_improve_proposal_actions = @(Get-PropertyValue -Object $proposalAcceptanceSummary -Name "actions")
            self_improve_proposal_action_plan_requires_validation_and_memory_admission = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_plan_requires_checked_passed_validation_and_accepted_memory_admission"
            self_improve_proposal_action_assignment_source = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_assignment_source"
            self_improve_proposal_action_assignment_target_count = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_assignment_target_count"
            self_improve_proposal_action_assignment_first_target = Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_assignment_first_target"
            self_improve_proposal_action_assignment_first_missing_requirements = @(Get-PropertyValue -Object $proposalAcceptanceSummary -Name "action_assignment_first_missing_requirements")
            self_improve_proposal_repair_factor_readiness_source = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "source"
            self_improve_proposal_repair_factor_readiness_action_required = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "action_required"
            self_improve_proposal_repair_factor_readiness_factor_count = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "repair_factor_count"
            self_improve_proposal_repair_factor_readiness_ready_count = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "ready_repair_factor_count"
            self_improve_proposal_repair_factor_readiness_blocked_count = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "blocked_count"
            self_improve_proposal_repair_factor_readiness_all_ready = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "all_repair_factors_ready"
            self_improve_proposal_repair_factor_readiness_first_factor = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "first_repair_factor_id"
            self_improve_proposal_repair_factor_readiness_first_ready = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "first_repair_factor_ready"
            self_improve_proposal_repair_factor_readiness_first_status = Get-PropertyValue -Object $proposalRepairFactorReadiness -Name "first_repair_factor_status"
            self_improve_proposal_repair_factor_release_source = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "source"
            self_improve_proposal_repair_factor_release_action_required = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "action_required"
            self_improve_proposal_repair_factor_release_factor_count = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "repair_factor_count"
            self_improve_proposal_repair_factor_release_release_count = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "release_count"
            self_improve_proposal_repair_factor_release_blocked_count = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "blocked_count"
            self_improve_proposal_repair_factor_release_ready = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "release_ready"
            self_improve_proposal_repair_factor_release_first_factor = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "first_repair_factor_id"
            self_improve_proposal_repair_factor_release_first_ready = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "first_release_ready"
            self_improve_proposal_repair_factor_release_first_status = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "first_release_status"
            self_improve_proposal_repair_factor_release_memory_store_write_allowed = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "memory_store_write_allowed"
            self_improve_proposal_repair_factor_release_ndkv_write_allowed = Get-PropertyValue -Object $proposalRepairFactorRelease -Name "ndkv_write_allowed"
            self_improve_proposal_repair_factor_retag_plan_source = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "source"
            self_improve_proposal_repair_factor_retag_plan_action_required = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "action_required"
            self_improve_proposal_repair_factor_retag_plan_factor_count = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "repair_factor_count"
            self_improve_proposal_repair_factor_retag_plan_count = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "retag_plan_count"
            self_improve_proposal_repair_factor_retag_plan_blocked_count = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "blocked_count"
            self_improve_proposal_repair_factor_retag_plan_ready = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "retag_plan_ready"
            self_improve_proposal_repair_factor_retag_plan_first_factor = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "first_repair_factor_id"
            self_improve_proposal_repair_factor_retag_plan_first_ready = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "first_retag_ready"
            self_improve_proposal_repair_factor_retag_plan_first_status = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "first_retag_status"
            self_improve_proposal_repair_factor_retag_plan_memory_store_write_allowed = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "memory_store_write_allowed"
            self_improve_proposal_repair_factor_retag_plan_ndkv_write_allowed = Get-PropertyValue -Object $proposalRepairFactorRetagPlan -Name "ndkv_write_allowed"
            self_improve_proposal_action_closure_source = Get-PropertyValue -Object $proposalActionClosure -Name "source"
            self_improve_proposal_action_closure_target_count = Get-PropertyValue -Object $proposalActionClosure -Name "target_count"
            self_improve_proposal_action_closure_closed_target_count = Get-PropertyValue -Object $proposalActionClosure -Name "closed_target_count"
            self_improve_proposal_action_closure_open_target_count = Get-PropertyValue -Object $proposalActionClosure -Name "open_target_count"
            self_improve_proposal_action_closure_first_target = Get-PropertyValue -Object $proposalActionClosure -Name "first_target"
            self_improve_proposal_action_closure_first_target_closed = Get-PropertyValue -Object $proposalActionClosure -Name "first_target_closed"
            self_improve_proposal_action_closure_first_target_closure_kind = Get-PropertyValue -Object $proposalActionClosure -Name "first_target_closure_kind"
            self_improve_proposal_action_closure_first_target_still_requires_memory_admission = Get-PropertyValue -Object $proposalActionClosure -Name "first_target_still_requires_memory_admission"
            self_improve_proposal_memory_admission_readiness_source = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "source"
            self_improve_proposal_memory_admission_readiness_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "target_count"
            self_improve_proposal_memory_admission_readiness_ready_count = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "ready_count"
            self_improve_proposal_memory_admission_readiness_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "blocked_count"
            self_improve_proposal_memory_admission_readiness_first_target = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "first_target"
            self_improve_proposal_memory_admission_readiness_first_target_ready = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "first_target_ready"
            self_improve_proposal_memory_admission_readiness_all_closed_targets_ready = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "all_closed_targets_ready"
            self_improve_proposal_memory_admission_readiness_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_readiness_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionReadiness -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_request_source = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "source"
            self_improve_proposal_memory_admission_request_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "target_count"
            self_improve_proposal_memory_admission_request_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "request_count"
            self_improve_proposal_memory_admission_request_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "blocked_count"
            self_improve_proposal_memory_admission_request_first_candidate = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "first_candidate"
            self_improve_proposal_memory_admission_request_first_candidate_ready = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "first_candidate_ready"
            self_improve_proposal_memory_admission_request_all_ready_targets_requested = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "all_ready_targets_requested"
            self_improve_proposal_memory_admission_request_writer_required = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "writer_required"
            self_improve_proposal_memory_admission_request_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "auto_apply"
            self_improve_proposal_memory_admission_request_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_request_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionRequest -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_decision_source = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "source"
            self_improve_proposal_memory_admission_decision_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "target_count"
            self_improve_proposal_memory_admission_decision_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "request_count"
            self_improve_proposal_memory_admission_decision_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "blocked_count"
            self_improve_proposal_memory_admission_decision_first_candidate = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "first_candidate"
            self_improve_proposal_memory_admission_decision_writer_required = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "writer_required"
            self_improve_proposal_memory_admission_decision_admission_writer_preflight_passed = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "admission_writer_preflight_passed"
            self_improve_proposal_memory_admission_decision_explicit_writer_invocation_required = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "explicit_writer_invocation_required"
            self_improve_proposal_memory_admission_decision_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_decision_gate_blocked = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "gate_blocked"
            self_improve_proposal_memory_admission_decision_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "failure_reasons")
            self_improve_proposal_memory_admission_decision_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "auto_apply"
            self_improve_proposal_memory_admission_decision_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_decision_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionDecision -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_writer_plan_source = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "source"
            self_improve_proposal_memory_admission_writer_plan_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "target_count"
            self_improve_proposal_memory_admission_writer_plan_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "request_count"
            self_improve_proposal_memory_admission_writer_plan_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "writer_plan_item_count"
            self_improve_proposal_memory_admission_writer_plan_ready_plan_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "ready_plan_count"
            self_improve_proposal_memory_admission_writer_plan_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "blocked_count"
            self_improve_proposal_memory_admission_writer_plan_first_plan_item = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "first_plan_item"
            self_improve_proposal_memory_admission_writer_plan_ready = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "writer_plan_ready"
            self_improve_proposal_memory_admission_writer_plan_explicit_writer_invocation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "explicit_writer_invocation_required"
            self_improve_proposal_memory_admission_writer_plan_experiment_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "experiment_required"
            self_improve_proposal_memory_admission_writer_plan_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "rollback_required"
            self_improve_proposal_memory_admission_writer_plan_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "validation_required"
            self_improve_proposal_memory_admission_writer_plan_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_writer_plan_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "failure_reasons")
            self_improve_proposal_memory_admission_writer_plan_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "auto_apply"
            self_improve_proposal_memory_admission_writer_plan_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_writer_plan_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterPlan -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_writer_dry_run_source = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "source"
            self_improve_proposal_memory_admission_writer_dry_run_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "target_count"
            self_improve_proposal_memory_admission_writer_dry_run_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "request_count"
            self_improve_proposal_memory_admission_writer_dry_run_plan_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "writer_plan_item_count"
            self_improve_proposal_memory_admission_writer_dry_run_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "dry_run_item_count"
            self_improve_proposal_memory_admission_writer_dry_run_ready_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "ready_dry_run_count"
            self_improve_proposal_memory_admission_writer_dry_run_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "blocked_count"
            self_improve_proposal_memory_admission_writer_dry_run_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "first_dry_run_item"
            self_improve_proposal_memory_admission_writer_dry_run_ready = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "dry_run_ready"
            self_improve_proposal_memory_admission_writer_dry_run_explicit_writer_invocation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "explicit_writer_invocation_required"
            self_improve_proposal_memory_admission_writer_dry_run_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "dry_run_required"
            self_improve_proposal_memory_admission_writer_dry_run_experiment_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "experiment_required"
            self_improve_proposal_memory_admission_writer_dry_run_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "rollback_required"
            self_improve_proposal_memory_admission_writer_dry_run_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "validation_required"
            self_improve_proposal_memory_admission_writer_dry_run_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_writer_dry_run_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "failure_reasons")
            self_improve_proposal_memory_admission_writer_dry_run_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "auto_apply"
            self_improve_proposal_memory_admission_writer_dry_run_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_writer_dry_run_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRun -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_source = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "source"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "target_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "request_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_dry_run_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "dry_run_item_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "receipt_item_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_succeeded_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "succeeded_receipt_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "blocked_count"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "first_receipt_item"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_ready = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "dry_run_receipt_ready"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_explicit_writer_invocation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "explicit_writer_invocation_required"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_commit_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "commit_allowed"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "validation_required"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "rollback_required"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "failure_reasons")
            self_improve_proposal_memory_admission_writer_dry_run_receipt_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "auto_apply"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_writer_dry_run_receipt_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionWriterDryRunReceipt -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_commit_record_stage_source = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "source"
            self_improve_proposal_memory_admission_commit_record_stage_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "target_count"
            self_improve_proposal_memory_admission_commit_record_stage_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "request_count"
            self_improve_proposal_memory_admission_commit_record_stage_receipt_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "receipt_item_count"
            self_improve_proposal_memory_admission_commit_record_stage_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "commit_record_item_count"
            self_improve_proposal_memory_admission_commit_record_stage_staged_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "staged_commit_record_count"
            self_improve_proposal_memory_admission_commit_record_stage_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "blocked_count"
            self_improve_proposal_memory_admission_commit_record_stage_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "first_commit_record_item"
            self_improve_proposal_memory_admission_commit_record_stage_ready = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "commit_record_stage_ready"
            self_improve_proposal_memory_admission_commit_record_stage_explicit_writer_invocation_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "explicit_writer_invocation_required"
            self_improve_proposal_memory_admission_commit_record_stage_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "validation_required"
            self_improve_proposal_memory_admission_commit_record_stage_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "rollback_required"
            self_improve_proposal_memory_admission_commit_record_stage_commit_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "commit_allowed"
            self_improve_proposal_memory_admission_commit_record_stage_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_commit_record_stage_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "failure_reasons")
            self_improve_proposal_memory_admission_commit_record_stage_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "auto_apply"
            self_improve_proposal_memory_admission_commit_record_stage_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_commit_record_stage_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitRecordStage -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_request_source = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "source"
            self_improve_proposal_memory_admission_commit_approval_request_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "target_count"
            self_improve_proposal_memory_admission_commit_approval_request_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "request_count"
            self_improve_proposal_memory_admission_commit_approval_request_commit_record_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "commit_record_item_count"
            self_improve_proposal_memory_admission_commit_approval_request_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "approval_request_item_count"
            self_improve_proposal_memory_admission_commit_approval_request_requested_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "requested_commit_approval_count"
            self_improve_proposal_memory_admission_commit_approval_request_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "blocked_count"
            self_improve_proposal_memory_admission_commit_approval_request_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "first_approval_request_item"
            self_improve_proposal_memory_admission_commit_approval_request_ready = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "commit_approval_request_ready"
            self_improve_proposal_memory_admission_commit_approval_request_explicit_commit_approval_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "explicit_commit_approval_required"
            self_improve_proposal_memory_admission_commit_approval_request_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "validation_required"
            self_improve_proposal_memory_admission_commit_approval_request_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "rollback_required"
            self_improve_proposal_memory_admission_commit_approval_request_commit_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "commit_allowed"
            self_improve_proposal_memory_admission_commit_approval_request_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_commit_approval_request_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "failure_reasons")
            self_improve_proposal_memory_admission_commit_approval_request_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "auto_apply"
            self_improve_proposal_memory_admission_commit_approval_request_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_request_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalRequest -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_decision_source = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "source"
            self_improve_proposal_memory_admission_commit_approval_decision_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "target_count"
            self_improve_proposal_memory_admission_commit_approval_decision_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "request_count"
            self_improve_proposal_memory_admission_commit_approval_decision_request_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "approval_request_item_count"
            self_improve_proposal_memory_admission_commit_approval_decision_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "approval_decision_item_count"
            self_improve_proposal_memory_admission_commit_approval_decision_recorded_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "recorded_approval_decision_count"
            self_improve_proposal_memory_admission_commit_approval_decision_approved_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "approved_commit_count"
            self_improve_proposal_memory_admission_commit_approval_decision_pending_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "pending_approval_count"
            self_improve_proposal_memory_admission_commit_approval_decision_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "blocked_count"
            self_improve_proposal_memory_admission_commit_approval_decision_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "first_approval_decision_item"
            self_improve_proposal_memory_admission_commit_approval_decision_ready = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "commit_approval_decision_ready"
            self_improve_proposal_memory_admission_commit_approval_decision_explicit_commit_approval_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "explicit_commit_approval_required"
            self_improve_proposal_memory_admission_commit_approval_decision_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "validation_required"
            self_improve_proposal_memory_admission_commit_approval_decision_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "rollback_required"
            self_improve_proposal_memory_admission_commit_approval_decision_commit_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "commit_allowed"
            self_improve_proposal_memory_admission_commit_approval_decision_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_commit_approval_decision_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "failure_reasons")
            self_improve_proposal_memory_admission_commit_approval_decision_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "auto_apply"
            self_improve_proposal_memory_admission_commit_approval_decision_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_decision_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalDecision -Name "ndkv_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_review_packet_source = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "source"
            self_improve_proposal_memory_admission_commit_approval_review_packet_target_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "target_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_request_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "request_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_request_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "approval_request_item_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_decision_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "approval_decision_item_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_item_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "review_packet_item_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_ready_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "ready_review_packet_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_pending_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "pending_approval_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_blocked_count = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "blocked_count"
            self_improve_proposal_memory_admission_commit_approval_review_packet_first_item = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "first_review_packet_item"
            self_improve_proposal_memory_admission_commit_approval_review_packet_ready = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "approval_review_packet_ready"
            self_improve_proposal_memory_admission_commit_approval_review_packet_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_admission_commit_approval_review_packet_validation_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "validation_required"
            self_improve_proposal_memory_admission_commit_approval_review_packet_rollback_required = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "rollback_required"
            self_improve_proposal_memory_admission_commit_approval_review_packet_commit_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "commit_allowed"
            self_improve_proposal_memory_admission_commit_approval_review_packet_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "admission_write_authorized"
            self_improve_proposal_memory_admission_commit_approval_review_packet_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "failure_reasons")
            self_improve_proposal_memory_admission_commit_approval_review_packet_auto_apply = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "auto_apply"
            self_improve_proposal_memory_admission_commit_approval_review_packet_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "memory_store_write_allowed"
            self_improve_proposal_memory_admission_commit_approval_review_packet_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryAdmissionCommitApprovalReviewPacket -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_usefulness_source = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "source"
            self_improve_proposal_memory_reflection_usefulness_target_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "target_count"
            self_improve_proposal_memory_reflection_usefulness_projected_report_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "projected_report_count"
            self_improve_proposal_memory_reflection_usefulness_accepted_memory_admission_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "accepted_memory_admission_count"
            self_improve_proposal_memory_reflection_usefulness_quarantined_candidate_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "quarantined_candidate_count"
            self_improve_proposal_memory_reflection_usefulness_review_packet_item_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "review_packet_item_count"
            self_improve_proposal_memory_reflection_usefulness_useful_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "useful_reflection_item_count"
            self_improve_proposal_memory_reflection_usefulness_pending_operator_approval_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "pending_operator_approval_count"
            self_improve_proposal_memory_reflection_usefulness_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "blocked_count"
            self_improve_proposal_memory_reflection_usefulness_wasted_compute_guard_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "wasted_compute_guard_count"
            self_improve_proposal_memory_reflection_usefulness_adapter_safe_count = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "adapter_safe_count"
            self_improve_proposal_memory_reflection_usefulness_first_item = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "first_reflection_item"
            self_improve_proposal_memory_reflection_usefulness_ready = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "reflection_usefulness_ready"
            self_improve_proposal_memory_reflection_usefulness_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_usefulness_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "validation_required"
            self_improve_proposal_memory_reflection_usefulness_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "rollback_required"
            self_improve_proposal_memory_reflection_usefulness_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "commit_allowed"
            self_improve_proposal_memory_reflection_usefulness_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_usefulness_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "failure_reasons")
            self_improve_proposal_memory_reflection_usefulness_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "auto_apply"
            self_improve_proposal_memory_reflection_usefulness_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_usefulness_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionUsefulness -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_dedupe_cluster_source = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "source"
            self_improve_proposal_memory_reflection_dedupe_cluster_target_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "target_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_useful_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "useful_reflection_item_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "reflection_cluster_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_wasted_compute_guard_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "wasted_compute_guard_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_pending_operator_approval_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "pending_operator_approval_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_adapter_safe_count = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "adapter_safe_count"
            self_improve_proposal_memory_reflection_dedupe_cluster_first_cluster = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "first_cluster_id"
            self_improve_proposal_memory_reflection_dedupe_cluster_ready = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "reflection_dedupe_ready"
            self_improve_proposal_memory_reflection_dedupe_cluster_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_dedupe_cluster_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "validation_required"
            self_improve_proposal_memory_reflection_dedupe_cluster_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "rollback_required"
            self_improve_proposal_memory_reflection_dedupe_cluster_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "commit_allowed"
            self_improve_proposal_memory_reflection_dedupe_cluster_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_dedupe_cluster_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "failure_reasons")
            self_improve_proposal_memory_reflection_dedupe_cluster_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "auto_apply"
            self_improve_proposal_memory_reflection_dedupe_cluster_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_dedupe_cluster_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionDedupeCluster -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_plan_source = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "source"
            self_improve_proposal_memory_reflection_reuse_plan_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_plan_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "reflection_cluster_count"
            self_improve_proposal_memory_reflection_reuse_plan_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "plan_item_count"
            self_improve_proposal_memory_reflection_reuse_plan_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "ready_reuse_plan_count"
            self_improve_proposal_memory_reflection_reuse_plan_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_plan_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_plan_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_plan_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "first_plan_item_id"
            self_improve_proposal_memory_reflection_reuse_plan_ready = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "reflection_reuse_plan_ready"
            self_improve_proposal_memory_reflection_reuse_plan_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_plan_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_plan_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_plan_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_plan_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_plan_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_plan_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_plan_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_plan_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePlan -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_preflight_source = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "source"
            self_improve_proposal_memory_reflection_reuse_preflight_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_preflight_plan_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "plan_item_count"
            self_improve_proposal_memory_reflection_reuse_preflight_ready_reuse_plan_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "ready_reuse_plan_count"
            self_improve_proposal_memory_reflection_reuse_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_preflight_passed_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "preflight_passed_item_count"
            self_improve_proposal_memory_reflection_reuse_preflight_blocked_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_preflight_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_preflight_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_preflight_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_preflight_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_preflight_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "first_preflight_item_id"
            self_improve_proposal_memory_reflection_reuse_preflight_passed = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "reuse_preflight_passed"
            self_improve_proposal_memory_reflection_reuse_preflight_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_preflight_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_preflight_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_preflight_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_preflight_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_preflight_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_preflight_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_preflight_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_preflight_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_preflight_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_preflight_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReusePreflight -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "lookup_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "ready_lookup_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_blocked_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_first_lookup_key = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "first_lookup_key"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "lookup_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupPreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "lookup_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_lookup_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "ready_lookup_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "approval_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "ready_approval_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_requested_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "requested_lookup_approval_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "first_approval_request_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "lookup_approval_request_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalRequest -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "lookup_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_lookup_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "ready_lookup_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approval_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "approval_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_approval_request_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "ready_approval_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "approval_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "ready_approval_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_pending_approval_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "pending_approval_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "first_approval_decision_preview_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "lookup_approval_decision_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalDecisionPreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "lookup_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_lookup_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "ready_lookup_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "approval_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_request_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "ready_approval_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "approval_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_decision_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "ready_approval_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "token_intake_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "ready_token_intake_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "first_token_intake_preview_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "lookup_approval_token_intake_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_preflight_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "preflight_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "lookup_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_lookup_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ready_lookup_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "approval_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_request_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ready_approval_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "approval_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_decision_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ready_approval_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_token_intake_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "token_intake_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_token_intake_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ready_token_intake_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "token_intake_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ready_token_intake_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "first_token_intake_decision_preview_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "lookup_approval_token_intake_decision_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_token_intake_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "token_intake_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_token_intake_decision_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "ready_token_intake_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "token_decision_record_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "ready_token_decision_record_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "first_token_decision_record_preview_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "lookup_approval_token_decision_record_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_token_decision_record_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "token_decision_record_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_token_decision_record_preview_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "ready_token_decision_record_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "token_decision_record_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "ready_token_decision_record_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_requested_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "requested_token_decision_record_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "first_token_decision_record_request_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "lookup_approval_token_decision_record_request_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequest -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_token_decision_record_request_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "token_decision_record_request_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_token_decision_record_request_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "ready_token_decision_record_request_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "token_decision_record_review_packet_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "ready_token_decision_record_review_packet_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_requested_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "requested_token_decision_record_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "first_token_decision_record_review_packet_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "lookup_approval_token_decision_record_review_packet_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacket -Name "ndkv_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_source = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "source"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_target_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "target_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_token_decision_record_review_packet_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "token_decision_record_review_packet_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_token_decision_record_review_packet_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "ready_token_decision_record_review_packet_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "token_decision_record_review_packet_decision_preview_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "ready_token_decision_record_review_packet_decision_preview_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_requested_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "requested_token_decision_record_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "pending_operator_token_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approved_lookup_execution_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "approved_lookup_execution_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_blocked_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "blocked_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "approval_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_cluster_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "duplicate_cluster_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_reflection_item_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "duplicate_reflection_item_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_saved_reflection_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "projected_saved_reflection_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_model_call_skip_count = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "projected_model_call_skip_count"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_first_item = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "first_token_decision_record_review_packet_decision_preview_id"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "lookup_approval_token_decision_record_review_packet_decision_preview_ready"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_validation_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "validation_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rollback_required = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "rollback_required"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_commit_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "commit_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "admission_write_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_model_call_skip_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "model_call_skip_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_execution_authorized = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "reflection_reuse_execution_authorized"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_lookup_performed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "memory_lookup_performed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_lookup_hit_assumed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "lookup_hit_assumed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "failure_reasons")
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_read_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "read_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "report_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_candidate_only = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "candidate_only"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_auto_apply = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "auto_apply"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreview -Name "ndkv_write_allowed"
            self_improve_proposal_memory_approval_token_intake_source = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "source"
            self_improve_proposal_memory_approval_token_intake_target_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "target_count"
            self_improve_proposal_memory_approval_token_intake_review_packet_item_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "review_packet_item_count"
            self_improve_proposal_memory_approval_token_intake_useful_reflection_item_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "useful_reflection_item_count"
            self_improve_proposal_memory_approval_token_intake_item_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "intake_item_count"
            self_improve_proposal_memory_approval_token_intake_ready_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "ready_intake_count"
            self_improve_proposal_memory_approval_token_intake_pending_operator_token_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "pending_operator_token_count"
            self_improve_proposal_memory_approval_token_intake_blocked_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "blocked_count"
            self_improve_proposal_memory_approval_token_intake_approval_token_present_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "approval_token_present_count"
            self_improve_proposal_memory_approval_token_intake_rejection_token_present_count = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "rejection_token_present_count"
            self_improve_proposal_memory_approval_token_intake_first_item = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "first_intake_item"
            self_improve_proposal_memory_approval_token_intake_ready = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "approval_token_intake_ready"
            self_improve_proposal_memory_approval_token_intake_explicit_operator_approval_required = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "explicit_operator_approval_required"
            self_improve_proposal_memory_approval_token_intake_validation_required = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "validation_required"
            self_improve_proposal_memory_approval_token_intake_rollback_required = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "rollback_required"
            self_improve_proposal_memory_approval_token_intake_commit_allowed = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "commit_allowed"
            self_improve_proposal_memory_approval_token_intake_admission_write_authorized = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "admission_write_authorized"
            self_improve_proposal_memory_approval_token_intake_failure_reasons = @(Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "failure_reasons")
            self_improve_proposal_memory_approval_token_intake_auto_apply = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "auto_apply"
            self_improve_proposal_memory_approval_token_intake_memory_store_write_allowed = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "memory_store_write_allowed"
            self_improve_proposal_memory_approval_token_intake_ndkv_write_allowed = Get-PropertyValue -Object $proposalMemoryApprovalTokenIntakePreview -Name "ndkv_write_allowed"
            ledger_lag_rounds = $lag
            stale = if ($null -eq $lag) { $null } else { [bool]($lag -gt 0) }
            remote_runtime_probed = Convert-ToNullableBool (Get-PropertyValue -Object $remoteRuntime -Name "probed")
            remote_runtime_acceleration_ok = Convert-ToNullableBool (Get-PropertyValue -Object $remoteRuntime -Name "acceleration_ok")
        }
    } catch {
        return [pscustomobject][ordered]@{
            path = $Path
            exists = $true
            parse_error = $_.Exception.Message
            rounds = $null
            success = $null
            failures = $null
            success_rate = $null
            report_gate_passed = $null
            report_gate_failure_count = $null
            self_improve_proposal_acceptance_summary_source = "unavailable"
            self_improve_proposal_business_count = $null
            self_improve_proposal_advisory_count = $null
            self_improve_proposal_repair_count = $null
            self_improve_proposal_accepted_without_business_evidence_count = $null
            self_improve_proposal_convert_advisory_to_business_evidence = $null
            self_improve_proposal_repair_unvalidated_or_unaccepted = $null
            self_improve_proposal_requires_validation_and_memory_admission = $null
            self_improve_proposal_action_required = $null
            self_improve_proposal_primary_action = $null
            self_improve_proposal_actions = @()
            self_improve_proposal_action_plan_requires_validation_and_memory_admission = $null
            self_improve_proposal_action_assignment_source = "unavailable"
            self_improve_proposal_action_assignment_target_count = $null
            self_improve_proposal_action_assignment_first_target = $null
            self_improve_proposal_action_assignment_first_missing_requirements = @()
            ledger_lag_rounds = $null
            stale = $null
            remote_runtime_probed = $null
            remote_runtime_acceleration_ok = $null
        }
    }
}

function New-StrictLedgerReportGateEvidence {
    param([object]$LedgerSummary)

    if (-not $StrictUnattendedEvolution) {
        return $null
    }

    $latest = if ($null -ne $LedgerSummary -and (Has-Property $LedgerSummary "latest")) { $LedgerSummary.latest } else { $null }
    $failures = @()
    if ($null -eq $latest -or -not $latest.success) {
        $failures += "latest_round_not_successful"
    }
    if (
        $null -eq $latest `
        -or -not $latest.validation_checked `
        -or -not $latest.validation_passed `
        -or [string]$latest.validation_command_source -ne "configured" `
        -or $latest.validation_status_code -ne 0
    ) {
        $failures += "latest_configured_validation_missing"
    }
    if (
        $null -eq $latest `
        -or -not $latest.success `
        -or $latest.feedback_applied -le 0 `
        -or -not $latest.self_improve_passed
    ) {
        $failures += "latest_self_improve_missing"
    }

    $requiredHelperRoles = Parse-CommaList -Value $RequireLatestHelperStageRolesEffective
    if ($requiredHelperRoles.Count -gt 0) {
        $presentHelperRoles = if ($null -ne $latest) { @($latest.helper_stage_roles) } else { @() }
        foreach ($role in $requiredHelperRoles) {
            if ($presentHelperRoles -notcontains $role) {
                $failures += "latest_helper_stage_roles_missing"
                break
            }
        }
    }
    if ($null -eq $latest -or -not $latest.helper_stage_contract_complete) {
        $failures += "latest_helper_stage_contract_incomplete"
    }
    if ($null -eq $latest -or -not $latest.test_gate_passed) {
        $failures += "latest_test_gate_not_pass"
    }
    if ($null -eq $latest -or [string]$latest.test_gate_validation_command_safety -ne "safe") {
        $failures += "latest_test_gate_validation_command_not_safe"
    }

    return [pscustomobject][ordered]@{
        source = "strict_unattended_ledger_latest"
        source_schema = "ledger_latest_round_summary_v1"
        source_path = "ledger.latest"
        source_round = if ($null -ne $latest -and (Has-Property $latest "round")) { $latest.round } else { $null }
        passed = $failures.Count -eq 0
        failure_count = $failures.Count
        failures = @($failures | Sort-Object -Unique)
        read_only = $true
        report_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
    }
}

function Read-LoopProcesses {
    if ($SkipProcess) {
        return [pscustomobject][ordered]@{
            checked = $false
            running = $null
            count = 0
            processes = @()
            error = ""
        }
    }

    $items = @()
    try {
        $processes = Get-CimInstance Win32_Process | Where-Object {
            $cmd = [string]$_.CommandLine
            ($cmd -match "evolution-loop\.exe|start-evolution-loop\.ps1") -and
                ($cmd -notmatch "status-evolution-loop")
        }
        foreach ($process in $processes) {
            $items += [pscustomobject][ordered]@{
                pid = [int]$process.ProcessId
                name = [string]$process.Name
                command_preview = ([string]$process.CommandLine).Substring(0, [Math]::Min(180, ([string]$process.CommandLine).Length))
            }
        }
    } catch {
        return [pscustomobject][ordered]@{
            checked = $false
            running = $false
            count = 0
            processes = @()
            error = $_.Exception.Message
        }
    }
    return [pscustomobject][ordered]@{
        checked = $true
        running = $items.Count -gt 0
        count = $items.Count
        processes = $items
        error = ""
    }
}

function Read-StatusFileLinesShared {
    param([string]$Path)

    $stream = $null
    $reader = $null
    try {
        $stream = [System.IO.File]::Open(
            $Path,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read,
            [System.IO.FileShare]::ReadWrite
        )
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8, $true)
        $lines = [System.Collections.Generic.List[string]]::new()
        while ($true) {
            $line = $reader.ReadLine()
            if ($null -eq $line) {
                break
            }
            [void]$lines.Add($line)
        }
        return @($lines.ToArray())
    } finally {
        if ($null -ne $reader) {
            $reader.Dispose()
        } elseif ($null -ne $stream) {
            $stream.Dispose()
        }
    }
}

function Read-StatusLogTail {
    param(
        [string]$Path,
        [int]$Count = 12
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }
    try {
        $lines = @(Read-StatusFileLinesShared -Path $Path)
    } catch {
        return @("log_read_error: $($_.Exception.Message)")
    }
    if ($lines.Count -le $Count) {
        return @($lines)
    }
    return @($lines[($lines.Count - $Count)..($lines.Count - 1)])
}

function Read-DaemonPidValue {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile -PathType Leaf)) {
        return $null
    }
    $text = (Get-Content -LiteralPath $PidFile -Raw).Trim()
    if ($text -notmatch "^\d+$") {
        return $null
    }
    return [int]$text
}

function Read-DaemonLatestRoundFromLedger {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    try {
        $lines = @(Read-StatusFileLinesShared -Path $Path)
    } catch {
        return $null
    }
    for ($i = $lines.Count - 1; $i -ge 0; $i -= 1) {
        $line = ([string]$lines[$i]).Trim()
        if ($line.Length -eq 0) {
            continue
        }
        try {
            $record = $line | ConvertFrom-Json
        } catch {
            continue
        }
        if (Has-Property $record "round") {
            return [int]$record.round
        }
    }
    return $null
}

function Get-StatusFileFreshness {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject][ordered]@{
            exists = $false
            age_seconds = $null
            length_bytes = 0
        }
    }
    $item = Get-Item -LiteralPath $Path
    return [pscustomobject][ordered]@{
        exists = $true
        age_seconds = [Math]::Max(0, [int][Math]::Floor(([DateTime]::UtcNow - $item.LastWriteTimeUtc).TotalSeconds))
        length_bytes = [int64]$item.Length
    }
}

function Convert-ToStatusUtcString {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    try {
        return ([DateTime]$Value).ToUniversalTime().ToString("o")
    } catch {
        return $null
    }
}

function Resolve-DaemonFreshnessPathList {
    param(
        [string]$EnvName,
        [string[]]$DefaultRelativePaths
    )

    $override = [Environment]::GetEnvironmentVariable($EnvName, "Process")
    $paths = @()
    if ($null -ne $override) {
        foreach ($rawPath in $override.Split(";")) {
            $trimmed = $rawPath.Trim()
            if ($trimmed.Length -eq 0) {
                continue
            }
            $paths += (Resolve-RepoPath $trimmed)
        }
        return @($paths)
    }

    foreach ($relativePath in $DefaultRelativePaths) {
        $paths += (Resolve-RepoPath $relativePath)
    }
    return @($paths)
}

function Get-DaemonSourceFreshnessPaths {
    $paths = @(Resolve-DaemonFreshnessPathList `
        -EnvName "NORION_DAEMON_SOURCE_FRESHNESS_PATHS" `
        -DefaultRelativePaths @(
            "tools\evolution-loop\daemon-evolution-loop.ps1",
            "tools\evolution-loop\status-evolution-loop.ps1",
            "tools\evolution-loop\Cargo.toml",
            "tools\evolution-loop\src\main.rs",
            "tools\evolution-loop\src\report.rs",
            "tools\evolution-loop\src\self_improve_proposal_artifact.rs"
        ))

    if ($null -eq [Environment]::GetEnvironmentVariable("NORION_DAEMON_SOURCE_FRESHNESS_PATHS", "Process")) {
        $gitHead = Resolve-RepoPath ".git\HEAD"
        $paths += $gitHead
        if (Test-Path -LiteralPath $gitHead -PathType Leaf) {
            try {
                $headText = (Get-Content -Raw -LiteralPath $gitHead).Trim()
                if ($headText.StartsWith("ref: ")) {
                    $refPath = $headText.Substring(5).Trim().Replace("/", "\")
                    if ($refPath.Length -gt 0) {
                        $paths += (Resolve-RepoPath (Join-Path ".git" $refPath))
                    }
                }
            } catch {
                # Best-effort source freshness only; missing git ref details should not break status.
            }
        }
    }

    return @($paths)
}

function Get-DaemonBinaryFreshnessPaths {
    return @(Resolve-DaemonFreshnessPathList `
        -EnvName "NORION_DAEMON_BINARY_FRESHNESS_PATHS" `
        -DefaultRelativePaths @(
            "target\debug\evolution-loop.exe",
            "tools\evolution-loop\target\debug\evolution-loop.exe"
        ))
}

function Get-LatestFreshnessPathWriteTime {
    param([string[]]$Paths)

    $latestPath = $null
    $latestWriteTimeUtc = $null
    $checkedPaths = @()
    $missingPaths = @()
    foreach ($path in @($Paths)) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            $missingPaths += $path
            continue
        }
        try {
            $item = Get-Item -LiteralPath $path
            $checkedPaths += [pscustomobject][ordered]@{
                path = $path
                last_write_time_utc = (Convert-ToStatusUtcString $item.LastWriteTimeUtc)
            }
            if ($null -eq $latestWriteTimeUtc -or $item.LastWriteTimeUtc -gt $latestWriteTimeUtc) {
                $latestPath = $path
                $latestWriteTimeUtc = $item.LastWriteTimeUtc
            }
        } catch {
            $missingPaths += $path
        }
    }

    return [pscustomobject][ordered]@{
        latest_path = $latestPath
        latest_write_time_utc_value = $latestWriteTimeUtc
        latest_write_time_utc = (Convert-ToStatusUtcString $latestWriteTimeUtc)
        checked_path_count = @($checkedPaths).Count
        missing_path_count = @($missingPaths).Count
        checked_paths = @($checkedPaths)
        missing_paths = @($missingPaths)
    }
}

function Get-DaemonSourceFreshness {
    param(
        [object]$PidValue,
        [bool]$Running
    )

    $processStartTimeUtc = $null
    $processStartError = ""
    if ($Running -and $null -ne $PidValue) {
        try {
            $process = Get-Process -Id $PidValue -ErrorAction Stop
            $processStartTimeUtc = $process.StartTime.ToUniversalTime()
        } catch {
            $processStartError = $_.Exception.Message
        }
    }

    $sourcePathsFromEnv = $null -ne [Environment]::GetEnvironmentVariable("NORION_DAEMON_SOURCE_FRESHNESS_PATHS", "Process")
    $binaryPathsFromEnv = $null -ne [Environment]::GetEnvironmentVariable("NORION_DAEMON_BINARY_FRESHNESS_PATHS", "Process")
    $sourceFreshness = Get-LatestFreshnessPathWriteTime -Paths @(Get-DaemonSourceFreshnessPaths)
    $binaryFreshness = Get-LatestFreshnessPathWriteTime -Paths @(Get-DaemonBinaryFreshnessPaths)

    $daemonStartedBeforeSourceUpdate = $null
    if ($null -ne $processStartTimeUtc -and $null -ne $sourceFreshness.latest_write_time_utc_value) {
        $daemonStartedBeforeSourceUpdate = [bool]($processStartTimeUtc -lt $sourceFreshness.latest_write_time_utc_value)
    }

    $daemonStartedBeforeBinaryUpdate = $null
    if ($null -ne $processStartTimeUtc -and $null -ne $binaryFreshness.latest_write_time_utc_value) {
        $daemonStartedBeforeBinaryUpdate = [bool]($processStartTimeUtc -lt $binaryFreshness.latest_write_time_utc_value)
    }

    $restartRecommended = $false
    $restartReason = "daemon_source_current"
    if (-not $Running) {
        $restartReason = "daemon_not_running"
    } elseif ($null -eq $processStartTimeUtc) {
        $restartReason = "daemon_process_start_time_unavailable"
    } elseif ($daemonStartedBeforeSourceUpdate -eq $true -and $daemonStartedBeforeBinaryUpdate -eq $true) {
        $restartRecommended = $true
        $restartReason = "daemon_started_before_source_and_binary_update"
    } elseif ($daemonStartedBeforeSourceUpdate -eq $true) {
        $restartRecommended = $true
        $restartReason = "daemon_started_before_source_update"
    } elseif ($daemonStartedBeforeBinaryUpdate -eq $true) {
        $restartRecommended = $true
        $restartReason = "daemon_started_before_binary_update"
    }

    return [pscustomobject][ordered]@{
        schema = "daemon_source_freshness_v1"
        checked = $true
        report_only = $true
        read_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        source_paths_from_env = [bool]$sourcePathsFromEnv
        binary_paths_from_env = [bool]$binaryPathsFromEnv
        daemon_pid = if ($Running) { $PidValue } else { $null }
        daemon_process_start_time_utc = (Convert-ToStatusUtcString $processStartTimeUtc)
        daemon_process_start_time_error = $processStartError
        source_latest_write_time_utc = $sourceFreshness.latest_write_time_utc
        source_latest_path = $sourceFreshness.latest_path
        source_checked_path_count = $sourceFreshness.checked_path_count
        source_missing_path_count = $sourceFreshness.missing_path_count
        source_checked_paths = @($sourceFreshness.checked_paths)
        source_missing_paths = @($sourceFreshness.missing_paths)
        binary_latest_write_time_utc = $binaryFreshness.latest_write_time_utc
        binary_latest_path = $binaryFreshness.latest_path
        binary_checked_path_count = $binaryFreshness.checked_path_count
        binary_missing_path_count = $binaryFreshness.missing_path_count
        daemon_started_before_source_update = $daemonStartedBeforeSourceUpdate
        daemon_started_before_binary_update = $daemonStartedBeforeBinaryUpdate
        daemon_restart_recommended = [bool]$restartRecommended
        restart_reason = $restartReason
    }
}

function Read-DaemonLogSummary {
    param([string]$LogPath)

    $latestRound = $null
    $latestStage = ""
    $latestEvent = ""
    $latestCompletedRound = $null
    $latestDoneRound = $null
    $latestRoundLine = ""
    $postRoundActivity = $false
    $postRoundActivityLine = ""
    $postRoundActivityEvent = ""
    $afterRoundCompletion = $false
    $roundState = "unknown"
    $roundInProgress = $false
    if (-not (Test-Path -LiteralPath $LogPath -PathType Leaf)) {
        return [pscustomobject][ordered]@{
            latest_round = $latestRound
            latest_stage = $latestStage
            latest_event = $latestEvent
            latest_completed_round = $latestCompletedRound
            post_round_activity = $postRoundActivity
            post_round_activity_event = $postRoundActivityEvent
            post_round_activity_line_preview = $postRoundActivityLine
            latest_round_state = $roundState
            round_in_progress = $roundInProgress
            latest_round_line_preview = $latestRoundLine
        }
    }
    try {
        $lines = @(Read-StatusFileLinesShared -Path $LogPath)
    } catch {
        $lines = @()
    }
    if ($lines.Count -gt 240) {
        $lines = @($lines[($lines.Count - 240)..($lines.Count - 1)])
    }
    foreach ($rawLine in $lines) {
        $line = [string]$rawLine
        if ($line -notmatch '^\[round\s+(\d+)\]\s+(.+)$') {
            if ($afterRoundCompletion -and $line.Trim().Length -gt 0) {
                $postRoundActivity = $true
                $postRoundActivityLine = $line
                $postRoundActivityEvent = ($line -split '\s+')[0]
            }
            continue
        }
        $round = [int]$Matches[1]
        $rest = [string]$Matches[2]
        $latestRound = $round
        $latestRoundLine = $line
        $completedThisLine = $false
        if ($rest -match '^stage\s+(.+)$') {
            $latestEvent = "stage"
            $latestStage = [string]$Matches[1]
            if ($latestStage -eq "ledger_append:done") {
                $latestCompletedRound = $round
                $completedThisLine = $true
            }
        } elseif ($rest -match '^ok\b') {
            $latestEvent = "ok"
            $latestCompletedRound = $round
            $completedThisLine = $true
        } elseif ($rest -match '^failed\b') {
            $latestEvent = "failed"
            $latestCompletedRound = $round
            $completedThisLine = $true
        } elseif ($rest -match '^done\b') {
            $latestEvent = "done"
            $latestDoneRound = $round
        } else {
            $latestEvent = ($rest -split '\s+')[0]
        }
        if ($completedThisLine) {
            $afterRoundCompletion = $true
        } else {
            $afterRoundCompletion = $false
            $postRoundActivity = $false
            $postRoundActivityLine = ""
            $postRoundActivityEvent = ""
        }
    }
    if ($null -ne $latestRound) {
        if ($null -ne $latestCompletedRound -and $latestCompletedRound -ge $latestRound) {
            $roundState = "completed"
        } elseif ($null -ne $latestDoneRound -and $latestDoneRound -ge $latestRound) {
            $roundState = "round_done_waiting_ledger_commit"
        } else {
            $roundState = "in_progress"
            $roundInProgress = $true
        }
    }
    if ($latestRoundLine.Length -gt 240) {
        $latestRoundLine = $latestRoundLine.Substring(0, 240)
    }
    if ($postRoundActivityLine.Length -gt 240) {
        $postRoundActivityLine = $postRoundActivityLine.Substring(0, 240)
    }
    return [pscustomobject][ordered]@{
        latest_round = $latestRound
        latest_stage = $latestStage
        latest_event = $latestEvent
        latest_completed_round = $latestCompletedRound
        latest_done_round = $latestDoneRound
        post_round_activity = $postRoundActivity
        post_round_activity_event = $postRoundActivityEvent
        post_round_activity_line_preview = $postRoundActivityLine
        latest_round_state = $roundState
        round_in_progress = $roundInProgress
        latest_round_line_preview = $latestRoundLine
    }
}

function Get-LaunchValidationSummary {
    param([object[]]$StderrTail)

    $launchLine = ""
    foreach ($rawLine in @($StderrTail)) {
        $line = [string]$rawLine
        if ($line -match '\bRunning\b' -or $line -match 'command=powershell\.exe') {
            $launchLine = $line
        }
    }

    $configuredRun = $launchLine.Contains("--require-configured-validation-run") -or $launchLine.Contains("-RequireConfiguredValidationRun")
    $testGateRun = $launchLine.Contains("--require-test-gate-validation-run") -or $launchLine.Contains("-RequireTestGateValidationRun")
    $validationCommand = $launchLine.Contains("--validation-command") -or $launchLine.Contains("-ValidationCommand")
    $useTestGateCommand = $launchLine.Contains("--use-test-gate-validation-command") -or $launchLine.Contains("-UseTestGateValidationCommand")
    $safeTestGateCommand = $launchLine.Contains("--require-safe-test-gate-validation-command") -or $launchLine.Contains("-RequireSafeTestGateValidationCommand")
    $testGatePass = $launchLine.Contains("--require-test-gate-pass") -or $launchLine.Contains("-RequireTestGatePass")

    $mode = "none"
    if ($configuredRun -and $testGateRun) {
        $mode = "mixed"
    } elseif ($configuredRun -or $validationCommand) {
        $mode = "configured"
    } elseif ($testGateRun -or $useTestGateCommand) {
        $mode = "test-gate"
    }

    $nextStep = "start daemon with -EnableConfiguredValidationRun or -EnableTestGateValidationRun when validation execution should be enforced"
    if ($mode -eq "configured") {
        $nextStep = "configured validation execution is requested by the daemon launch command"
    } elseif ($mode -eq "test-gate") {
        $nextStep = "test-gate validation execution is requested by the daemon launch command"
    } elseif ($mode -eq "mixed") {
        $nextStep = "restart with only one validation source; configured and test-gate validation gates are mutually exclusive"
    } elseif ($launchLine.Trim().Length -eq 0) {
        $nextStep = "launch command was not found in stderr tail; inspect daemon stderr log"
    }

    $preview = $launchLine
    if ($preview.Length -gt 360) {
        $preview = $preview.Substring(0, 360)
    }

    return [pscustomobject][ordered]@{
        launch_command_seen = $launchLine.Trim().Length -gt 0
        mode = $mode
        validation_command_present = $validationCommand
        use_test_gate_validation_command = $useTestGateCommand
        require_configured_validation_run = $configuredRun
        require_test_gate_validation_run = $testGateRun
        require_safe_test_gate_validation_command = $safeTestGateCommand
        require_test_gate_pass = $testGatePass
        validation_execution_enforced = $configuredRun -or $testGateRun
        launch_command_preview = $preview
        next_step = $nextStep
    }
}

function Get-DaemonTransitionKind {
    param(
        [string]$ActivityState,
        [string]$LatestRoundState
    )

    if (($ActivityState -eq "active" -or $ActivityState -eq "slow_in_progress") -and $LatestRoundState -eq "in_progress") {
        return "normal_in_progress"
    }
    if ($ActivityState -eq "round_done_waiting_ledger_commit") {
        return "round_done_waiting_ledger_commit"
    }
    if ($ActivityState.StartsWith("stale") -or $ActivityState -eq "in_progress_no_stdout") {
        return "stale_no_activity"
    }
    return $ActivityState
}

function New-DaemonRoundTransitionStatus {
    param(
        [string]$ActivityState,
        [bool]$ActivityOk,
        [string]$ActivityReason,
        [object]$ActiveRound,
        [object]$LedgerLatestRound,
        [object]$LedgerLagRounds,
        [string]$LatestRoundState,
        [object]$LatestDoneRound,
        [bool]$RoundInProgress,
        [object]$StdoutAgeSeconds,
        [object]$LedgerAgeSeconds,
        [int]$MaxInProgressStdoutAgeSeconds,
        [int]$MaxRoundTimeoutSeconds,
        [int]$MaxIdleLedgerAgeSeconds
    )

    $withinRequestTimeout = $false
    if ($null -ne $StdoutAgeSeconds) {
        $withinRequestTimeout = [int]$StdoutAgeSeconds -le [Math]::Max(1, $MaxRoundTimeoutSeconds)
    }

    return [pscustomobject][ordered]@{
        schema = "daemon_round_transition_status_v1"
        transition_kind = Get-DaemonTransitionKind -ActivityState $ActivityState -LatestRoundState $LatestRoundState
        activity_state = $ActivityState
        activity_ok = $ActivityOk
        activity_reason = $ActivityReason
        active_round = $ActiveRound
        ledger_latest_round = $LedgerLatestRound
        ledger_lag_rounds = $LedgerLagRounds
        latest_round_state = $LatestRoundState
        latest_done_round = $LatestDoneRound
        round_in_progress = $RoundInProgress
        stdout_age_seconds = $StdoutAgeSeconds
        ledger_age_seconds = $LedgerAgeSeconds
        max_in_progress_stdout_age_seconds = $MaxInProgressStdoutAgeSeconds
        max_round_timeout_seconds = $MaxRoundTimeoutSeconds
        within_request_timeout = [bool]$withinRequestTimeout
        max_idle_ledger_age_seconds = $MaxIdleLedgerAgeSeconds
        read_only = $true
        starts_process = $false
        sends_prompt = $false
    }
}

function New-LiveStatusBundle {
    param(
        [object]$DaemonStatus,
        [object]$ReportStatus,
        [object]$RemoteChainStatus,
        [object]$LedgerReportGateEvidence = $null
    )

    $reportGatePassedFromReport = if ($null -ne $ReportStatus -and (Has-Property $ReportStatus "report_gate_passed")) { $ReportStatus.report_gate_passed } else { $null }
    $reportGateFailureCountFromReport = if ($null -ne $ReportStatus -and (Has-Property $ReportStatus "report_gate_failure_count")) { $ReportStatus.report_gate_failure_count } else { $null }
    $hasReportGateEvidence = $null -ne $reportGatePassedFromReport
    $reportGateSource = if ($hasReportGateEvidence) { "report_json" } elseif ($null -ne $LedgerReportGateEvidence) { $LedgerReportGateEvidence.source } else { "unavailable" }
    $reportGateSourcePath = if ($hasReportGateEvidence) { "report.report_gate" } elseif ($null -ne $LedgerReportGateEvidence) { $LedgerReportGateEvidence.source_path } else { $null }
    $reportGateSourceRound = if ($null -ne $LedgerReportGateEvidence -and (Has-Property $LedgerReportGateEvidence "source_round")) { $LedgerReportGateEvidence.source_round } else { $null }
    $reportGateFailures = if ($null -ne $LedgerReportGateEvidence -and -not $hasReportGateEvidence) { @($LedgerReportGateEvidence.failures) } else { @() }

    return [pscustomobject][ordered]@{
        schema = "live_status_bundle_v1"
        report_only = $true
        read_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        daemon = [pscustomobject][ordered]@{
            state = if ($null -ne $DaemonStatus -and (Has-Property $DaemonStatus "activity_state")) { [string]$DaemonStatus.activity_state } else { "" }
            daemon_round_transition_status = if ($null -ne $DaemonStatus -and (Has-Property $DaemonStatus "daemon_round_transition_status")) { $DaemonStatus.daemon_round_transition_status } else { $null }
        }
        report_gate = [pscustomobject][ordered]@{
            passed = if ($hasReportGateEvidence) { $reportGatePassedFromReport } elseif ($null -ne $LedgerReportGateEvidence) { $LedgerReportGateEvidence.passed } else { $null }
            failure_count = if ($hasReportGateEvidence) { $reportGateFailureCountFromReport } elseif ($null -ne $LedgerReportGateEvidence) { $LedgerReportGateEvidence.failure_count } else { $null }
            source = $reportGateSource
            source_path = $reportGateSourcePath
            source_round = $reportGateSourceRound
            failures = @($reportGateFailures)
        }
        remote_pool = [pscustomobject][ordered]@{
            healthy = if ($null -ne $RemoteChainStatus -and (Has-Property $RemoteChainStatus "ready")) { $RemoteChainStatus.ready } else { $null }
            healthy_workers = if ($null -ne $RemoteChainStatus -and (Has-Property $RemoteChainStatus "healthy_worker_count")) { $RemoteChainStatus.healthy_worker_count } else { $null }
            required_workers = if ($null -ne $RemoteChainStatus -and (Has-Property $RemoteChainStatus "worker_count")) { $RemoteChainStatus.worker_count } else { $null }
        }
    }
}

function New-NextRoundDecision {
    param(
        [object]$LiveStatusBundle
    )

    $transition = Get-NestedValue -Value $LiveStatusBundle -Path @("daemon", "daemon_round_transition_status")
    $reportGate = Get-NestedValue -Value $LiveStatusBundle -Path @("report_gate")
    $transitionKind = if ($null -ne $transition -and (Has-Property $transition "transition_kind")) { [string]$transition.transition_kind } else { "" }
    $roundInProgress = if ($null -ne $transition -and (Has-Property $transition "round_in_progress")) { $transition.round_in_progress } else { $null }
    $activityOk = if ($null -ne $transition -and (Has-Property $transition "activity_ok")) { $transition.activity_ok } else { $null }
    $reportGatePassed = if ($null -ne $reportGate -and (Has-Property $reportGate "passed")) { $reportGate.passed } else { $null }
    $reportGateFailureCount = if ($null -ne $reportGate -and (Has-Property $reportGate "failure_count")) { $reportGate.failure_count } else { $null }
    $withinRequestTimeout = if ($null -ne $transition -and (Has-Property $transition "within_request_timeout")) { $transition.within_request_timeout } else { $null }
    $stdoutAgeSeconds = if ($null -ne $transition -and (Has-Property $transition "stdout_age_seconds")) { $transition.stdout_age_seconds } else { $null }
    $maxRoundTimeoutSeconds = if ($null -ne $transition -and (Has-Property $transition "max_round_timeout_seconds")) { $transition.max_round_timeout_seconds } else { $null }
    $reportGateAllowsActiveWait = $reportGatePassed -eq $true -or (
        $null -eq $reportGatePassed `
            -and -not [bool]$StrictUnattendedEvolution `
            -and $transitionKind -eq "normal_in_progress" `
            -and $roundInProgress -eq $true `
            -and $activityOk -eq $true
    )

    $safeToContinueTransitionKinds = @(
        "round_done_waiting_ledger_commit",
        "post_round_activity",
        "idle_completed"
    )
    $safeToWait = $reportGateAllowsActiveWait -and $transitionKind -eq "normal_in_progress" -and $roundInProgress -eq $true -and $activityOk -eq $true
    $safeToContinue = $reportGatePassed -eq $true -and ($safeToContinueTransitionKinds -contains $transitionKind) -and $roundInProgress -eq $false -and $activityOk -eq $true
    $operatorBlocked = -not ($safeToWait -or $safeToContinue)

    $displayState = "blocked-operator-attention"
    $reasonCode = "operator_attention_required_until_safe_next_round_evidence_present"
    if ($safeToWait) {
        $displayState = "safe-to-wait"
        if ($null -eq $reportGatePassed) {
            $reasonCode = "active_round_in_progress_report_gate_unavailable_non_strict_wait"
        } else {
            $reasonCode = "active_round_in_progress_wait_for_completion"
        }
    } elseif ($safeToContinue) {
        $displayState = "safe-to-continue-after-current-round"
        if ($transitionKind -eq "idle_completed") {
            $reasonCode = "idle_completed_report_gate_passed_ready_for_next_round"
        } elseif ($transitionKind -eq "post_round_activity") {
            $reasonCode = "post_round_activity_report_gate_passed_ready_for_next_round"
        } else {
            $reasonCode = "done_marker_seen_wait_for_ledger_commit_then_continue"
        }
    } elseif ($reportGatePassed -eq $false) {
        $reasonCode = "report_gate_failed_operator_attention_required"
    } elseif ($null -eq $reportGatePassed) {
        $reasonCode = "report_gate_unavailable_operator_attention_required"
    } elseif ([string]::IsNullOrWhiteSpace($transitionKind)) {
        $reasonCode = "transition_status_unavailable_operator_attention_required"
    } else {
        $reasonCode = "transition_state_not_safe_for_next_round_display"
    }

    return [pscustomobject][ordered]@{
        schema = "next_round_decision_evidence_v1"
        consumer_surface = "evolution_loop_status_report_next_round_decision"
        report_only = $true
        read_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        changes_daemon_loop_behavior = $false
        changes_prompt_content = $false
        changes_report_gate_stop_semantics = $false
        changes_runtime_calls = $false
        changes_model_pool_behavior = $false
        display_state = $displayState
        safe_to_wait_current_round_active = [bool]$safeToWait
        safe_to_continue_after_current_round = [bool]$safeToContinue
        operator_attention_blocked = [bool]$operatorBlocked
        operator_attention_required = [bool]$operatorBlocked
        may_display_unattended_continuation = [bool]($safeToWait -or $safeToContinue)
        wait_for_current_round = [bool]$safeToWait
        continue_after_current_round = [bool]$safeToContinue
        reason_code = $reasonCode
        evidence = [pscustomobject][ordered]@{
            transition_kind = $transitionKind
            report_gate_passed = $reportGatePassed
            report_gate_failure_count = $reportGateFailureCount
            report_gate_allows_active_wait = [bool]$reportGateAllowsActiveWait
            active_busy = [bool]($transitionKind -eq "normal_in_progress")
            round_in_progress = $roundInProgress
            activity_ok = $activityOk
            within_request_timeout = $withinRequestTimeout
            stdout_age_seconds = $stdoutAgeSeconds
            max_round_timeout_seconds = $maxRoundTimeoutSeconds
        }
    }
}

function New-NextRoundDecisionReportV1 {
    param(
        [object]$NextRoundDecision
    )

    return [pscustomobject][ordered]@{
        schema = "next_round_decision_report_v1"
        consumer_surface = "evolution_loop_status_report_next_round_decision"
        source_schema = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "schema")) { $NextRoundDecision.schema } else { $null }
        report_only = $true
        read_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        changes_daemon_loop_behavior = $false
        changes_prompt_content = $false
        changes_report_gate_stop_semantics = $false
        changes_runtime_calls = $false
        changes_model_pool_behavior = $false
        display_state = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "display_state")) { $NextRoundDecision.display_state } else { "blocked-operator-attention" }
        safe_to_wait_current_round_active = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "safe_to_wait_current_round_active")) { [bool]$NextRoundDecision.safe_to_wait_current_round_active } else { $false }
        safe_to_continue_after_current_round = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "safe_to_continue_after_current_round")) { [bool]$NextRoundDecision.safe_to_continue_after_current_round } else { $false }
        operator_attention_blocked = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "operator_attention_blocked")) { [bool]$NextRoundDecision.operator_attention_blocked } else { $true }
        operator_attention_required = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "operator_attention_required")) { [bool]$NextRoundDecision.operator_attention_required } else { $true }
        may_display_unattended_continuation = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "may_display_unattended_continuation")) { [bool]$NextRoundDecision.may_display_unattended_continuation } else { $false }
        wait_for_current_round = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "wait_for_current_round")) { [bool]$NextRoundDecision.wait_for_current_round } else { $false }
        continue_after_current_round = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "continue_after_current_round")) { [bool]$NextRoundDecision.continue_after_current_round } else { $false }
        reason_code = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "reason_code")) { $NextRoundDecision.reason_code } else { "operator_attention_required_until_safe_next_round_evidence_present" }
        evidence = if ($null -ne $NextRoundDecision -and (Has-Property $NextRoundDecision "evidence")) { $NextRoundDecision.evidence } else { $null }
        next_round_decision = $NextRoundDecision
    }
}

function Get-NextRoundDownstreamDecisionStatus {
    param([object]$NextRoundDecisionReportV1)

    $displayState = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "display_state")) { [string]$NextRoundDecisionReportV1.display_state } else { "" }
    switch ($displayState) {
        "safe-to-wait" { return "safe_to_wait_current_round_active" }
        "safe-to-continue-after-current-round" { return "safe_to_continue_after_current_round" }
        default { return "operator_attention_blocked" }
    }
}

function New-NextRoundDownstreamStatusConsumersV1 {
    param(
        [object]$NextRoundDecisionReportV1,
        [object]$DaemonRoundTransitionStatus = $null
    )

    $sourceDecisionStatus = Get-NextRoundDownstreamDecisionStatus -NextRoundDecisionReportV1 $NextRoundDecisionReportV1
    $operatorAttentionRequired = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "operator_attention_required")) { [bool]$NextRoundDecisionReportV1.operator_attention_required } else { $true }
    $effectiveDecisionStatus = if ($operatorAttentionRequired) { "operator_attention_blocked" } else { $sourceDecisionStatus }

    $serviceCliDisplayStatus = "display_operator_attention"
    $forgeOperatorDisplayStatus = "forge_operator_attention"
    $agentAssignmentAcceptance = "reject_until_operator_attention"
    $memorySelfImproveAdmissionVisibility = "visible_operator_attention_blocked"
    if ($effectiveDecisionStatus -eq "safe_to_wait_current_round_active") {
        $serviceCliDisplayStatus = "display_safe_to_wait_current_round"
        $forgeOperatorDisplayStatus = "forge_safe_to_wait"
        $agentAssignmentAcceptance = "defer_until_current_round_completes"
        $memorySelfImproveAdmissionVisibility = "visible_waiting_current_round"
    } elseif ($effectiveDecisionStatus -eq "safe_to_continue_after_current_round") {
        $serviceCliDisplayStatus = "display_safe_to_continue"
        $forgeOperatorDisplayStatus = "forge_safe_to_continue"
        $agentAssignmentAcceptance = "accept_next_round_assignment"
        $memorySelfImproveAdmissionVisibility = "visible_admission_safe"
    }

    $readOnly = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "read_only")) { [bool]$NextRoundDecisionReportV1.read_only } else { $false }
    $reportOnly = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "report_only")) { [bool]$NextRoundDecisionReportV1.report_only } else { $false }
    $sideEffects = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "side_effects")) { [bool]$NextRoundDecisionReportV1.side_effects } else { $true }
    $startsProcess = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "starts_process")) { [bool]$NextRoundDecisionReportV1.starts_process } else { $true }
    $sendsPrompt = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "sends_prompt")) { [bool]$NextRoundDecisionReportV1.sends_prompt } else { $true }
    $noSideEffects = (-not $sideEffects) -and (-not $startsProcess) -and (-not $sendsPrompt)
    $displayState = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "display_state")) { $NextRoundDecisionReportV1.display_state } else { "blocked-operator-attention" }
    $failureReasons = @()
    if ($operatorAttentionRequired -and $null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "reason_code")) {
        $failureReasons += [string]$NextRoundDecisionReportV1.reason_code
    }
    $roundEvidenceSourceSchema = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "schema")) { $DaemonRoundTransitionStatus.schema } else { $null }
    $roundEvidenceSourcePath = if ($null -ne $DaemonRoundTransitionStatus) { "daemon.daemon_round_transition_status" } else { $null }
    $activeRound = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "active_round")) { $DaemonRoundTransitionStatus.active_round } else { $null }
    $ledgerLatestRound = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "ledger_latest_round")) { $DaemonRoundTransitionStatus.ledger_latest_round } else { $null }
    $latestDoneRound = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "latest_done_round")) { $DaemonRoundTransitionStatus.latest_done_round } else { $null }
    $ledgerLagRounds = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "ledger_lag_rounds")) { $DaemonRoundTransitionStatus.ledger_lag_rounds } else { $null }
    $transitionKind = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "transition_kind")) { $DaemonRoundTransitionStatus.transition_kind } else { $null }
    $latestRoundState = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "latest_round_state")) { $DaemonRoundTransitionStatus.latest_round_state } else { $null }
    $roundInProgress = if ($null -ne $DaemonRoundTransitionStatus -and (Has-Property $DaemonRoundTransitionStatus "round_in_progress")) { $DaemonRoundTransitionStatus.round_in_progress } else { $null }
    $hasRoundIdEvidence = $null -ne $activeRound -or $null -ne $ledgerLatestRound -or $null -ne $latestDoneRound

    return [pscustomobject][ordered]@{
        schema = "next_round_downstream_status_consumers_v1"
        consumer_surface = "evolution_loop_status_report_downstream_next_round_consumers"
        source_schema = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "schema")) { $NextRoundDecisionReportV1.schema } else { $null }
        report_only = $true
        read_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        changes_daemon_loop_behavior = $false
        changes_prompt_content = $false
        changes_report_gate_stop_semantics = $false
        changes_runtime_calls = $false
        changes_model_pool_behavior = $false
        normalized_facts = $true
        consumers = [pscustomobject][ordered]@{
            service_cli_display_status = $true
            forge_operator_display = $true
            agent_assignment_acceptance = $true
            memory_self_improve_admission_visibility = $true
        }
        next_round_downstream = [pscustomobject][ordered]@{
            source_decision_status = $sourceDecisionStatus
            effective_decision_status = $effectiveDecisionStatus
            service_cli_display_status = $serviceCliDisplayStatus
            forge_operator_display_status = $forgeOperatorDisplayStatus
            agent_assignment_acceptance = $agentAssignmentAcceptance
            memory_self_improve_admission_visibility = $memorySelfImproveAdmissionVisibility
            operator_attention_required = [bool]$operatorAttentionRequired
            read_only = [bool]$readOnly
            report_only = [bool]$reportOnly
            no_side_effects = [bool]$noSideEffects
            dispatch_work_allowed = $false
            prompt_replay_allowed = $false
            process_start_allowed = $false
            memory_write_allowed = $false
            ndkv_write_allowed = $false
            current_round_active = if ($null -ne $NextRoundDecisionReportV1 -and (Has-Property $NextRoundDecisionReportV1 "safe_to_wait_current_round_active")) { [bool]$NextRoundDecisionReportV1.safe_to_wait_current_round_active } else { $false }
            live_status_display_state = $displayState
            active_round = $activeRound
            ledger_latest_round = $ledgerLatestRound
            latest_done_round = $latestDoneRound
            round_id_evidence = [pscustomobject][ordered]@{
                source_schema = $roundEvidenceSourceSchema
                source_path = $roundEvidenceSourcePath
                has_round_id_evidence = [bool]$hasRoundIdEvidence
                active_round = $activeRound
                ledger_latest_round = $ledgerLatestRound
                latest_done_round = $latestDoneRound
                ledger_lag_rounds = $ledgerLagRounds
                transition_kind = $transitionKind
                latest_round_state = $latestRoundState
                round_in_progress = $roundInProgress
                read_only = $true
                report_only = $true
                side_effects = $false
                starts_process = $false
                sends_prompt = $false
            }
            readiness_can_schedule_next_round = [bool]($effectiveDecisionStatus -eq "safe_to_continue_after_current_round")
            failure_reasons = @($failureReasons)
        }
        next_round_decision_report_v1 = $NextRoundDecisionReportV1
    }
}

function Read-DaemonSnapshot {
    param([string]$WorkDirPath)

    $validationExecutionRequired = [bool]$RequireDaemonValidationExecutionEffective
    if ($SkipDaemon) {
        return [pscustomobject][ordered]@{
            checked = $false
            running = $null
            validation_execution_required = $validationExecutionRequired
            validation_execution_ok = -not $validationExecutionRequired
            validation_execution_failure = if ($validationExecutionRequired) { "daemon status was skipped" } else { "" }
            launch_validation = Get-LaunchValidationSummary -StderrTail @()
            source_freshness = [pscustomobject][ordered]@{
                schema = "daemon_source_freshness_v1"
                checked = $false
                report_only = $true
                read_only = $true
                side_effects = $false
                starts_process = $false
                sends_prompt = $false
                daemon_restart_recommended = $false
                restart_reason = "daemon_status_skipped"
            }
            operator_summary = ""
            error = ""
        }
    }
    $pidFile = Join-Path $WorkDirPath "evolution-loop.pid"
    $stdoutLog = Join-Path $WorkDirPath "evolution-loop.out.log"
    $stderrLog = Join-Path $WorkDirPath "evolution-loop.err.log"
    $ledgerPath = Join-Path $WorkDirPath "evolution-ledger.jsonl"
    $pidValue = Read-DaemonPidValue -PidFile $pidFile
    $running = $false
    if ($null -ne $pidValue) {
        try {
            $null = Get-Process -Id $pidValue -ErrorAction Stop
            $running = $true
        } catch {
            $running = $false
        }
    }
    $stalePidFile = (Test-Path -LiteralPath $pidFile -PathType Leaf) -and $null -ne $pidValue -and -not $running
    $stderrTail = Read-StatusLogTail -Path $stderrLog
    $launchValidation = Get-LaunchValidationSummary -StderrTail $stderrTail
    $logSummary = Read-DaemonLogSummary -LogPath $stdoutLog
    $ledgerRound = Read-DaemonLatestRoundFromLedger -Path $ledgerPath
    $activeRound = $logSummary.latest_round
    $lag = $null
    if ($null -ne $activeRound -and $null -ne $ledgerRound) {
        $lag = [Math]::Max(0, [int]$activeRound - [int]$ledgerRound)
    }
    $stdoutFreshness = Get-StatusFileFreshness -Path $stdoutLog
    $ledgerFreshness = Get-StatusFileFreshness -Path $ledgerPath
    $sourceFreshness = Get-DaemonSourceFreshness -PidValue $pidValue -Running $running
    $inProgressStdoutMaxAge = [Math]::Max(1, [int]$MaxDaemonInProgressStdoutAgeSecondsEffective)
    $roundTimeoutMaxAge = [Math]::Max($inProgressStdoutMaxAge, [int]$MaxDaemonRoundTimeoutSecondsEffective)
    $idleLedgerMaxAge = [Math]::Max(0, [int]$MaxDaemonIdleLedgerAgeSecondsEffective)
    $state = "unknown"
    $ok = $false
    $reason = "no_round_evidence"
    $nextStep = "inspect daemon stdout/stderr logs"
    if (-not $running) {
        $state = if ($stalePidFile) { "stale_pid" } else { "not_running" }
        $reason = if ($stalePidFile) { "pid_file_points_to_missing_process" } else { "daemon_process_not_running" }
        $nextStep = if ($stalePidFile) { "remove stale pid file or restart daemon" } else { "start daemon when unattended evolution should run" }
    } elseif ($logSummary.round_in_progress -eq $true) {
        if ($stdoutFreshness.age_seconds -ne $null -and [int]$stdoutFreshness.age_seconds -le $inProgressStdoutMaxAge) {
            $state = "active"
            $ok = $true
            $reason = "round_in_progress_stdout_recent"
            $nextStep = "wait for current round to finish or inspect log_preview"
        } elseif ($stdoutFreshness.age_seconds -ne $null -and [int]$stdoutFreshness.age_seconds -le $roundTimeoutMaxAge) {
            $state = "slow_in_progress"
            $ok = $true
            $reason = "round_in_progress_within_request_timeout"
            $nextStep = "wait for current round until request timeout before treating it as stale"
        } else {
            $state = "stale_in_progress"
            $reason = "round_in_progress_stdout_stale"
            $nextStep = "check backend health, model worker, and daemon stdout"
        }
    } elseif ($logSummary.latest_round_state -eq "round_done_waiting_ledger_commit") {
        if ($null -ne $lag -and [int]$lag -gt 0) {
            $state = "round_done_waiting_ledger_commit"
            $reason = "stdout_done_marker_seen_waiting_for_ledger_commit"
            if ($stdoutFreshness.age_seconds -ne $null -and [int]$stdoutFreshness.age_seconds -le $inProgressStdoutMaxAge) {
                $ok = $true
            }
            $nextStep = "wait for ledger commit for the round marked done in stdout"
        } else {
            $state = "round_done_waiting_ledger_commit"
            $reason = "stdout_done_marker_seen_without_ledger_commit_evidence"
            $nextStep = "inspect ledger append and daemon stdout; done marker was seen without a newer ledger record"
        }
    } elseif ($logSummary.latest_round_state -eq "completed") {
        if ($null -ne $lag -and [int]$lag -gt 0) {
            $state = "ledger_lag_after_completion"
            $reason = "latest_round_completed_but_ledger_lag_remains"
            $nextStep = "inspect ledger append and report gate output"
        } elseif ($logSummary.post_round_activity -eq $true) {
            if ($stdoutFreshness.age_seconds -ne $null -and [int]$stdoutFreshness.age_seconds -le $inProgressStdoutMaxAge) {
                $state = "post_round_activity"
                $ok = $true
                $reason = "post_round_activity_stdout_recent"
                $nextStep = "wait for post-run gates or the next interval"
            } else {
                $state = "stale_post_round_activity"
                $reason = "post_round_activity_stdout_stale"
                $nextStep = "post-run activity stopped updating; inspect daemon stdout/stderr and backend health"
            }
        } else {
            if ($idleLedgerMaxAge -gt 0 -and $ledgerFreshness.age_seconds -ne $null -and [int]$ledgerFreshness.age_seconds -gt $idleLedgerMaxAge) {
                $state = "stale_idle_completed"
                $reason = "idle_completed_ledger_stale"
                $nextStep = "daemon is idle but ledger is older than freshness threshold; inspect stdout/stderr or restart at a safe point"
            } else {
                $state = "idle_completed"
                $ok = $true
                $reason = "latest_round_completed_and_ledger_current"
                $nextStep = "wait for next interval or inspect latest ledger round"
            }
        }
    }
    $stage = if ([string]$logSummary.latest_stage -ne "") { [string]$logSummary.latest_stage } else { [string]$logSummary.latest_event }
    $stdoutAge = if ($null -ne $stdoutFreshness.age_seconds) { "$($stdoutFreshness.age_seconds)s" } else { "unknown" }
    $ledgerAge = if ($null -ne $ledgerFreshness.age_seconds) { "$($ledgerFreshness.age_seconds)s" } else { "unknown" }
    $activeText = if ($null -ne $activeRound) { [string]$activeRound } else { "unknown" }
    $ledgerText = if ($null -ne $ledgerRound) { [string]$ledgerRound } else { "unknown" }
    $lagText = if ($null -ne $lag) { [string]$lag } else { "unknown" }
    $transitionStatus = New-DaemonRoundTransitionStatus `
        -ActivityState $state `
        -ActivityOk $ok `
        -ActivityReason $reason `
        -ActiveRound $activeRound `
        -LedgerLatestRound $ledgerRound `
        -LedgerLagRounds $lag `
        -LatestRoundState $logSummary.latest_round_state `
        -LatestDoneRound $logSummary.latest_done_round `
        -RoundInProgress $logSummary.round_in_progress `
        -StdoutAgeSeconds $stdoutFreshness.age_seconds `
        -LedgerAgeSeconds $ledgerFreshness.age_seconds `
        -MaxInProgressStdoutAgeSeconds $inProgressStdoutMaxAge `
        -MaxRoundTimeoutSeconds $roundTimeoutMaxAge `
        -MaxIdleLedgerAgeSeconds $idleLedgerMaxAge
    return [pscustomobject][ordered]@{
        checked = $true
        work_dir = $WorkDirPath
        running = $running
        pid = if ($running) { $pidValue } else { $null }
        stale_pid_file = $stalePidFile
        stale_pid = if ($stalePidFile) { $pidValue } else { $null }
        active_round = $activeRound
        ledger_latest_round = $ledgerRound
        ledger_lag_rounds = $lag
        activity_state = $state
        activity_ok = $ok
        activity_reason = $reason
        activity_next_step = $nextStep
        daemon_round_transition_status = $transitionStatus
        source_freshness = $sourceFreshness
        source_freshness_checked = $sourceFreshness.checked
        daemon_restart_recommended = $sourceFreshness.daemon_restart_recommended
        daemon_restart_reason = $sourceFreshness.restart_reason
        latest_stage = $logSummary.latest_stage
        latest_round_state = $logSummary.latest_round_state
        latest_done_round = $logSummary.latest_done_round
        round_in_progress = $logSummary.round_in_progress
        post_round_activity = $logSummary.post_round_activity
        post_round_activity_event = $logSummary.post_round_activity_event
        post_round_activity_line_preview = $logSummary.post_round_activity_line_preview
        stdout_age_seconds = $stdoutFreshness.age_seconds
        ledger_age_seconds = $ledgerFreshness.age_seconds
        max_in_progress_stdout_age_seconds = $inProgressStdoutMaxAge
        max_round_timeout_seconds = $roundTimeoutMaxAge
        max_idle_ledger_age_seconds = $idleLedgerMaxAge
        latest_round_line_preview = $logSummary.latest_round_line_preview
        operator_summary = "state=$state ok=$ok reason=$reason active_round=$activeText ledger_round=$ledgerText lag=$lagText stage=$stage stdout_age=$stdoutAge ledger_age=$ledgerAge next_step=$nextStep"
        validation_execution_required = $validationExecutionRequired
        validation_execution_ok = (-not $validationExecutionRequired) -or ($running -and $launchValidation.validation_execution_enforced)
        validation_execution_failure = if ($validationExecutionRequired -and -not $running) { "daemon is not running" } elseif ($validationExecutionRequired -and -not $launchValidation.validation_execution_enforced) { "daemon launch command does not enforce validation execution" } else { "" }
        launch_validation = $launchValidation
        error = ""
    }
}

$DaemonWorkDirPath = Resolve-RepoPath $DaemonWorkDir
$daemonStatus = Read-DaemonSnapshot -WorkDirPath $DaemonWorkDirPath
$DaemonLedgerAutoSelected = $false
$DaemonLedgerCandidate = Join-Path $DaemonWorkDir "evolution-ledger.jsonl"
$DaemonLedgerCandidatePath = Resolve-RepoPath $DaemonLedgerCandidate
if (
    -not $UseDaemonLedgerEffective `
        -and -not $PSBoundParameters.ContainsKey("Ledger") `
        -and -not [bool]$SkipDaemon `
        -and $null -ne $daemonStatus `
        -and $daemonStatus.running -eq $true `
        -and (Test-Path -LiteralPath $DaemonLedgerCandidatePath)
) {
    $UseDaemonLedgerEffective = $true
    $DaemonLedgerAutoSelected = $true
}
if ($UseDaemonLedgerEffective) {
    $Ledger = Join-Path $DaemonWorkDir "evolution-ledger.jsonl"
}
$DaemonReportAutoSelected = $false
$DaemonReportCandidate = Join-Path $DaemonWorkDir "report.json"
$DaemonReportCandidatePath = Resolve-RepoPath $DaemonReportCandidate
if (
    -not $PSBoundParameters.ContainsKey("ReportJson") `
        -and [string]::IsNullOrWhiteSpace($ReportJson) `
        -and -not [bool]$SkipDaemon `
        -and $null -ne $daemonStatus `
        -and (
            $daemonStatus.running -eq $true `
                -or (
                    (Has-Property $daemonStatus "activity_ok") `
                        -and $daemonStatus.activity_ok -eq $true
                )
        ) `
        -and (Test-Path -LiteralPath $DaemonReportCandidatePath)
) {
    $ReportJson = $DaemonReportCandidate
    $DaemonReportAutoSelected = $true
}
$LedgerPath = Resolve-RepoPath $Ledger
$ReportPath = Resolve-RepoPath $ReportJson
$RemoteStatusPath = Resolve-RepoPath $RemoteChainStatusJson
$ledgerRead = Read-Ledger -Path $LedgerPath
$ledgerSummary = Ledger-Summary -Path $LedgerPath -Records @($ledgerRead.records) -InvalidRecords $ledgerRead.invalid_records
$backendHealth = Read-BackendHealth -Backend $Backend
$remoteChain = Read-RemoteChainStatus -Path $RemoteStatusPath
$processStatus = Read-LoopProcesses
$reportStatus = Read-ReportStatus -Path $ReportPath -LedgerSummary $ledgerSummary
$ledgerReportGateEvidence = New-StrictLedgerReportGateEvidence -LedgerSummary $ledgerSummary
$liveStatusBundle = New-LiveStatusBundle -DaemonStatus $daemonStatus -ReportStatus $reportStatus -RemoteChainStatus $remoteChain -LedgerReportGateEvidence $ledgerReportGateEvidence
$nextRoundDecision = New-NextRoundDecision -LiveStatusBundle $liveStatusBundle
$nextRoundDecisionReportV1 = New-NextRoundDecisionReportV1 -NextRoundDecision $nextRoundDecision
$daemonRoundTransitionStatus = if ($null -ne $daemonStatus -and (Has-Property $daemonStatus "daemon_round_transition_status")) { $daemonStatus.daemon_round_transition_status } else { $null }
$nextRoundDownstreamStatusConsumersV1 = New-NextRoundDownstreamStatusConsumersV1 -NextRoundDecisionReportV1 $nextRoundDecisionReportV1 -DaemonRoundTransitionStatus $daemonRoundTransitionStatus
$liveStatusBundle | Add-Member -NotePropertyName "next_round_decision" -NotePropertyValue $nextRoundDecision
$liveStatusBundle | Add-Member -NotePropertyName "next_round_decision_report_v1" -NotePropertyValue $nextRoundDecisionReportV1
$liveStatusBundle | Add-Member -NotePropertyName "next_round_downstream_status_consumers_v1" -NotePropertyValue $nextRoundDownstreamStatusConsumersV1
$backendHealthDegraded = $backendHealth.checked -and (Has-Property $backendHealth "health_degraded") -and [bool]$backendHealth.health_degraded
$backendBusyDuringActiveDaemon = Test-BackendBusyDuringActiveDaemon -BackendHealth $backendHealth -DaemonStatus $daemonStatus

$failures = @()
if (-not $ledgerSummary.exists) {
    $failures += "ledger_missing"
}
if ($ledgerSummary.total_records -lt $MinRounds) {
    $failures += "rounds_below_minimum"
}
if ($ledgerSummary.invalid_records -gt 0) {
    $failures += "ledger_has_invalid_records"
}
if ($StrictLedgerHygiene -and ($ledgerSummary.duplicate_rounds -gt 0 -or $ledgerSummary.non_monotonic_rounds -gt 0 -or $ledgerSummary.round_gaps -gt 0)) {
    $failures += "strict_ledger_hygiene_failed"
}
if ($ledgerSummary.feedback_applied_total -lt $MinFeedbackTotal) {
    $failures += "feedback_below_minimum"
}
if ($null -eq $ledgerSummary.latest -or -not $ledgerSummary.latest.success) {
    $failures += "latest_round_not_successful"
}
if (
    $backendHealth.checked `
    -and (-not $backendHealth.ok -or -not $backendHealth.readiness_ok -or -not $backendHealth.safe_device_ok) `
    -and -not $backendBusyDuringActiveDaemon
) {
    $failures += "backend_not_ready"
}
if ($remoteChain.checked -and -not $remoteChain.ready) {
    $failures += "remote_chain_not_ready"
}
if ($RequireDaemonHealthyEffective) {
    if (-not $daemonStatus.checked -or -not $daemonStatus.running -or -not $daemonStatus.activity_ok) {
        $failures += "daemon_not_healthy"
    }
}
if ($RequireDaemonValidationExecutionEffective) {
    if (-not $daemonStatus.checked -or -not $daemonStatus.validation_execution_ok) {
        $failures += "daemon_validation_execution_missing"
    }
}
if ($RequireLatestConfiguredValidationRunEffective) {
    $latest = $ledgerSummary.latest
    if (
        $null -eq $latest `
        -or -not $latest.validation_checked `
        -or -not $latest.validation_passed `
        -or [string]$latest.validation_command_source -ne "configured" `
        -or $latest.validation_status_code -ne 0
    ) {
        $failures += "latest_configured_validation_missing"
    }
}
if ($RequireLatestSelfImproveEffective) {
    $latest = $ledgerSummary.latest
    if (
        $null -eq $latest `
        -or -not $latest.success `
        -or $latest.feedback_applied -le 0 `
        -or -not $latest.self_improve_passed
    ) {
        $failures += "latest_self_improve_missing"
    }
}
if ($RequireLatestHelperStageRolesEffective.Trim().Length -gt 0) {
    $latest = $ledgerSummary.latest
    $requiredHelperRoles = Parse-CommaList -Value $RequireLatestHelperStageRolesEffective
    $presentHelperRoles = if ($null -ne $latest) { @($latest.helper_stage_roles) } else { @() }
    $missingHelperRoles = @()
    foreach ($role in $requiredHelperRoles) {
        if ($presentHelperRoles -notcontains $role) {
            $missingHelperRoles += $role
        }
    }
    if ($null -eq $latest -or $missingHelperRoles.Count -gt 0) {
        $failures += "latest_helper_stage_roles_missing"
    }
}
if ($RequireLatestHelperStageContractsEffective) {
    $latest = $ledgerSummary.latest
    if ($null -eq $latest -or -not $latest.helper_stage_contract_complete) {
        $failures += "latest_helper_stage_contract_incomplete"
    }
}
if ($RequireLatestTestGatePassEffective) {
    $latest = $ledgerSummary.latest
    if ($null -eq $latest -or -not $latest.test_gate_passed) {
        $failures += "latest_test_gate_not_pass"
    }
}
if ($RequireLatestSafeTestGateValidationCommandEffective) {
    $latest = $ledgerSummary.latest
    if ($null -eq $latest -or [string]$latest.test_gate_validation_command_safety -ne "safe") {
        $failures += "latest_test_gate_validation_command_not_safe"
    }
}

$nextStep = if ($failures.Count -gt 0) {
    "fix status failures before unattended evolution"
} elseif ($daemonStatus.checked -and (Has-Property $daemonStatus "daemon_restart_recommended") -and $daemonStatus.daemon_restart_recommended) {
    if ((Has-Property $daemonStatus "round_in_progress") -and $daemonStatus.round_in_progress -eq $true) {
        "daemon source stale: wait for current round, then restart daemon before next round ($($daemonStatus.daemon_restart_reason))"
    } else {
        "daemon source stale: restart daemon before next round ($($daemonStatus.daemon_restart_reason))"
    }
} elseif ($daemonStatus.checked -and $daemonStatus.running) {
    "daemon $($daemonStatus.activity_state): $($daemonStatus.activity_next_step)"
} elseif ($backendHealthDegraded) {
    "ready via model_pool_status fallback; investigate /health latency if recurring"
} else {
    "ready: run budgeted -Forever or inspect report gate"
}

$status = [pscustomobject][ordered]@{
    schema_version = 1
    contract_version = "smartsteam.evolution-loop.status.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    strict_unattended_evolution = [bool]$StrictUnattendedEvolution
    repo = $RepoRoot
    backend_endpoint = $Backend
    ledger_source = if ($DaemonLedgerAutoSelected) { "daemon_auto" } elseif ($UseDaemonLedgerEffective) { "daemon" } else { "argument" }
    daemon_ledger_auto_selected = $DaemonLedgerAutoSelected
    report_source = if ($DaemonReportAutoSelected) { "daemon_auto" } elseif (-not [string]::IsNullOrWhiteSpace($ReportJson)) { "argument" } else { "none" }
    daemon_report_auto_selected = $DaemonReportAutoSelected
    ledger = $ledgerSummary
    report = $reportStatus
    backend = $backendHealth
    remote_chain = $remoteChain
    process = $processStatus
    daemon = $daemonStatus
    live_status_bundle = $liveStatusBundle
    next_round_decision = $nextRoundDecision
    next_round_decision_report_v1 = $nextRoundDecisionReportV1
    next_round_downstream_status_consumers_v1 = $nextRoundDownstreamStatusConsumersV1
    readiness = [pscustomobject][ordered]@{
        ready = $failures.Count -eq 0
        failures = $failures
        backend_busy_during_active_daemon = [bool]$backendBusyDuringActiveDaemon
    }
    next_step = $nextStep
}

$StatusExitCode = if ($FailOnNotReady -and -not $status.readiness.ready) { 2 } else { 0 }

if ($JsonStatus) {
    $status | ConvertTo-Json -Depth 10
    exit $StatusExitCode
}

Write-Host "SmartSteam evolution-loop status"
Write-Host "read_only=true starts_process=false sends_prompt=false"
Write-Host "profile: strict_unattended_evolution=$([bool]$StrictUnattendedEvolution)"
Write-Host "repo: $RepoRoot"
Write-Host "ledger: $LedgerPath source=$($status.ledger_source)"
Write-Host "status: ready=$($status.readiness.ready) failures=$($status.readiness.failures -join ',')"
if ($processStatus.checked) {
    Write-Host "process: checked=true running=$($processStatus.running) count=$($processStatus.count)"
} else {
    Write-Host "process: checked=false count=$($processStatus.count) error=$($processStatus.error)"
}
if ($daemonStatus.checked) {
    Write-Host "daemon: running=$($daemonStatus.running) state=$($daemonStatus.activity_state) ok=$($daemonStatus.activity_ok) active_round=$($daemonStatus.active_round) ledger_round=$($daemonStatus.ledger_latest_round) lag=$($daemonStatus.ledger_lag_rounds) summary=$($daemonStatus.operator_summary)"
    Write-Host "daemon_transition: schema=$($daemonStatus.daemon_round_transition_status.schema) kind=$($daemonStatus.daemon_round_transition_status.transition_kind) state=$($daemonStatus.daemon_round_transition_status.activity_state) ok=$($daemonStatus.daemon_round_transition_status.activity_ok) reason=$($daemonStatus.daemon_round_transition_status.activity_reason) active_round=$($daemonStatus.daemon_round_transition_status.active_round) ledger_round=$($daemonStatus.daemon_round_transition_status.ledger_latest_round) lag=$($daemonStatus.daemon_round_transition_status.ledger_lag_rounds) latest_round_state=$($daemonStatus.daemon_round_transition_status.latest_round_state) round_in_progress=$($daemonStatus.daemon_round_transition_status.round_in_progress)"
    Write-Host "daemon_source_freshness: checked=$($daemonStatus.source_freshness.checked) restart_recommended=$($daemonStatus.source_freshness.daemon_restart_recommended) reason=$($daemonStatus.source_freshness.restart_reason) process_start=$($daemonStatus.source_freshness.daemon_process_start_time_utc) source_latest=$($daemonStatus.source_freshness.source_latest_write_time_utc) source_latest_path=$($daemonStatus.source_freshness.source_latest_path) binary_latest=$($daemonStatus.source_freshness.binary_latest_write_time_utc) binary_latest_path=$($daemonStatus.source_freshness.binary_latest_path) read_only=$($daemonStatus.source_freshness.read_only) starts_process=$($daemonStatus.source_freshness.starts_process) sends_prompt=$($daemonStatus.source_freshness.sends_prompt)"
    Write-Host "launch_validation: mode=$($daemonStatus.launch_validation.mode) enforced=$($daemonStatus.launch_validation.validation_execution_enforced) configured_run=$($daemonStatus.launch_validation.require_configured_validation_run) test_gate_run=$($daemonStatus.launch_validation.require_test_gate_validation_run) validation_command=$($daemonStatus.launch_validation.validation_command_present) use_test_gate_command=$($daemonStatus.launch_validation.use_test_gate_validation_command) next_step=$($daemonStatus.launch_validation.next_step)"
    if ($daemonStatus.validation_execution_required) {
        Write-Host "daemon_validation_execution_gate: required=$($daemonStatus.validation_execution_required) ok=$($daemonStatus.validation_execution_ok) failure=$($daemonStatus.validation_execution_failure)"
    }
}
Write-Host "ledger_records: total=$($ledgerSummary.total_records) unique_rounds=$($ledgerSummary.unique_rounds) duplicate_rounds=$($ledgerSummary.duplicate_rounds) non_monotonic_rounds=$($ledgerSummary.non_monotonic_rounds) round_gaps=$($ledgerSummary.round_gaps)"
Write-Host "success: $($ledgerSummary.success_count)/$($ledgerSummary.total_records) ($($ledgerSummary.success_rate)%) feedback_total=$($ledgerSummary.feedback_applied_total) runtime_tokens_total=$($ledgerSummary.runtime_tokens_total)"
if ($null -ne $ledgerSummary.latest) {
    $latestHelperRoles = @($ledgerSummary.latest.helper_stage_roles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    $latestHelperContractIncomplete = @($ledgerSummary.latest.helper_stage_contract_incomplete_roles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    Write-Host "latest: round=$($ledgerSummary.latest.round) case=$($ledgerSummary.latest.case) success=$($ledgerSummary.latest.success) tokens=$($ledgerSummary.latest.runtime_tokens) elapsed_ms=$($ledgerSummary.latest.elapsed_ms) feedback=$($ledgerSummary.latest.feedback_applied) self_improve=$($ledgerSummary.latest.self_improve_passed) validation_checked=$($ledgerSummary.latest.validation_checked) validation_passed=$($ledgerSummary.latest.validation_passed) validation_source=$($ledgerSummary.latest.validation_command_source) validation_status=$($ledgerSummary.latest.validation_status_code) helper_stage_roles=$($latestHelperRoles -join ',') helper_stage_feedback_total=$($ledgerSummary.latest.helper_stage_feedback_total) helper_stage_contract_complete=$($ledgerSummary.latest.helper_stage_contract_complete) helper_stage_contract_incomplete_roles=$($latestHelperContractIncomplete -join ',') test_gate_verdict=$($ledgerSummary.latest.test_gate_verdict) test_gate_validation_command_safety=$($ledgerSummary.latest.test_gate_validation_command_safety)"
}
if ($backendHealth.checked) {
    Write-Host "backend: ok=$($backendHealth.ok) readiness_ok=$($backendHealth.readiness_ok) safe_device_ok=$($backendHealth.safe_device_ok) busy=$($backendHealth.engine_busy) active=$($backendHealth.active_engine_requests) model=$($backendHealth.gemma_runtime_model) source=$($backendHealth.source) fallback=$($backendHealth.health_fallback_used) degraded=$($backendHealth.health_degraded) health_error=$($backendHealth.health_error) error=$($backendHealth.error)"
}
if ($remoteChain.checked) {
    $remoteRuntimeText = ""
    if ($null -ne $remoteChain.remote_runtime) {
        $cpuOrNoGpuRoles = @($remoteChain.remote_runtime.cpu_or_no_gpu_roles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
        $metadataMayDifferRoles = @($remoteChain.remote_runtime.backend_metadata_may_differ_roles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
        $remoteRuntimeText = " remote_runtime_probed=$($remoteChain.remote_runtime.probed) remote_runtime_workers=$($remoteChain.remote_runtime.worker_count) remote_runtime_cpu_or_no_gpu=$($remoteChain.remote_runtime.cpu_or_no_gpu_count) remote_runtime_cpu_or_no_gpu_roles=$($cpuOrNoGpuRoles -join ',') remote_runtime_backend_metadata_may_differ_roles=$($metadataMayDifferRoles -join ',') remote_runtime_acceleration_ok=$($remoteChain.remote_runtime.acceleration_ok) remote_runtime_next_step=$($remoteChain.remote_runtime.acceleration_next_step) remote_runtime_error=$($remoteChain.remote_runtime.error)"
    }
    Write-Host "remote_chain: ready=$($remoteChain.ready) model_api=$($remoteChain.model_api) backend=$($remoteChain.backend) web_lab=$($remoteChain.web_lab) workers=$($remoteChain.healthy_worker_count)/$($remoteChain.worker_count) model_cache_ok=$($remoteChain.model_cache_ok_count)/$($remoteChain.model_cache_model_count) model_cache_all_ok=$($remoteChain.model_cache_all_ok) model_cache_remote_errors=$($remoteChain.model_cache_remote_error_count) error=$($remoteChain.error)$remoteRuntimeText"
}
if ($ReportPath.Trim().Length -gt 0) {
    Write-Host "report: exists=$($reportStatus.exists) path=$($reportStatus.path) rounds=$($reportStatus.rounds) ledger_lag=$($reportStatus.ledger_lag_rounds) stale=$($reportStatus.stale) success=$($reportStatus.success) failures=$($reportStatus.failures) success_rate=$($reportStatus.success_rate) gate_passed=$($reportStatus.report_gate_passed) gate_failures=$($reportStatus.report_gate_failure_count) remote_runtime_probed=$($reportStatus.remote_runtime_probed) remote_runtime_acceleration_ok=$($reportStatus.remote_runtime_acceleration_ok) parse_error=$($reportStatus.parse_error)"
    $proposalActions = @($reportStatus.self_improve_proposal_actions | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $proposalActionsText = $proposalActions -join ","
    $proposalFirstMissing = @($reportStatus.self_improve_proposal_action_assignment_first_missing_requirements | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_acceptance_summary_v1: source=$($reportStatus.self_improve_proposal_acceptance_summary_source) evidence_backed_business=$($reportStatus.self_improve_proposal_business_count) advisory_only=$($reportStatus.self_improve_proposal_advisory_count) repair_required=$($reportStatus.self_improve_proposal_repair_count) accepted_without_business_evidence=$($reportStatus.self_improve_proposal_accepted_without_business_evidence_count) convert_advisory_to_business_evidence=$($reportStatus.self_improve_proposal_convert_advisory_to_business_evidence) repair_unvalidated_or_unaccepted=$($reportStatus.self_improve_proposal_repair_unvalidated_or_unaccepted) requires_validation_and_memory_admission=$($reportStatus.self_improve_proposal_requires_validation_and_memory_admission) action_required=$($reportStatus.self_improve_proposal_action_required) primary_action=$($reportStatus.self_improve_proposal_primary_action) actions=$proposalActionsText action_plan_requires_validation_and_memory_admission=$($reportStatus.self_improve_proposal_action_plan_requires_validation_and_memory_admission) action_assignment_source=$($reportStatus.self_improve_proposal_action_assignment_source) action_assignment_targets=$($reportStatus.self_improve_proposal_action_assignment_target_count) action_assignment_first_target=$($reportStatus.self_improve_proposal_action_assignment_first_target) action_assignment_first_missing=$proposalFirstMissing"
    Write-Host "report_self_improve_proposal_repair_factor_readiness_report_v1: source=$($reportStatus.self_improve_proposal_repair_factor_readiness_source) action_required=$($reportStatus.self_improve_proposal_repair_factor_readiness_action_required) factors=$($reportStatus.self_improve_proposal_repair_factor_readiness_factor_count) ready=$($reportStatus.self_improve_proposal_repair_factor_readiness_ready_count) blocked=$($reportStatus.self_improve_proposal_repair_factor_readiness_blocked_count) all_ready=$($reportStatus.self_improve_proposal_repair_factor_readiness_all_ready) first_factor=$($reportStatus.self_improve_proposal_repair_factor_readiness_first_factor) first_ready=$($reportStatus.self_improve_proposal_repair_factor_readiness_first_ready) first_status=$($reportStatus.self_improve_proposal_repair_factor_readiness_first_status)"
    Write-Host "report_self_improve_proposal_repair_factor_release_report_v1: source=$($reportStatus.self_improve_proposal_repair_factor_release_source) action_required=$($reportStatus.self_improve_proposal_repair_factor_release_action_required) factors=$($reportStatus.self_improve_proposal_repair_factor_release_factor_count) releases=$($reportStatus.self_improve_proposal_repair_factor_release_release_count) blocked=$($reportStatus.self_improve_proposal_repair_factor_release_blocked_count) release_ready=$($reportStatus.self_improve_proposal_repair_factor_release_ready) first_factor=$($reportStatus.self_improve_proposal_repair_factor_release_first_factor) first_ready=$($reportStatus.self_improve_proposal_repair_factor_release_first_ready) first_status=$($reportStatus.self_improve_proposal_repair_factor_release_first_status) memory_store_write_allowed=$($reportStatus.self_improve_proposal_repair_factor_release_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_repair_factor_release_ndkv_write_allowed)"
    Write-Host "report_self_improve_proposal_repair_factor_retag_plan_v1: source=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_source) action_required=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_action_required) factors=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_factor_count) retag_plans=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_count) blocked=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_blocked_count) retag_plan_ready=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_ready) first_factor=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_first_factor) first_ready=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_first_ready) first_status=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_first_status) memory_store_write_allowed=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_repair_factor_retag_plan_ndkv_write_allowed)"
    Write-Host "report_self_improve_proposal_action_closure_report_v1: source=$($reportStatus.self_improve_proposal_action_closure_source) targets=$($reportStatus.self_improve_proposal_action_closure_target_count) closed=$($reportStatus.self_improve_proposal_action_closure_closed_target_count) open=$($reportStatus.self_improve_proposal_action_closure_open_target_count) first_target=$($reportStatus.self_improve_proposal_action_closure_first_target) first_closed=$($reportStatus.self_improve_proposal_action_closure_first_target_closed) first_kind=$($reportStatus.self_improve_proposal_action_closure_first_target_closure_kind) first_still_requires_memory_admission=$($reportStatus.self_improve_proposal_action_closure_first_target_still_requires_memory_admission)"
    Write-Host "report_self_improve_proposal_memory_admission_readiness_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_readiness_source) targets=$($reportStatus.self_improve_proposal_memory_admission_readiness_target_count) ready=$($reportStatus.self_improve_proposal_memory_admission_readiness_ready_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_readiness_blocked_count) first_target=$($reportStatus.self_improve_proposal_memory_admission_readiness_first_target) first_ready=$($reportStatus.self_improve_proposal_memory_admission_readiness_first_target_ready) all_closed_targets_ready=$($reportStatus.self_improve_proposal_memory_admission_readiness_all_closed_targets_ready) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_readiness_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_readiness_ndkv_write_allowed)"
    Write-Host "report_self_improve_proposal_memory_admission_request_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_request_source) targets=$($reportStatus.self_improve_proposal_memory_admission_request_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_request_request_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_request_blocked_count) first_candidate=$($reportStatus.self_improve_proposal_memory_admission_request_first_candidate) first_ready=$($reportStatus.self_improve_proposal_memory_admission_request_first_candidate_ready) all_ready_targets_requested=$($reportStatus.self_improve_proposal_memory_admission_request_all_ready_targets_requested) writer_required=$($reportStatus.self_improve_proposal_memory_admission_request_writer_required) auto_apply=$($reportStatus.self_improve_proposal_memory_admission_request_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_request_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_request_ndkv_write_allowed)"
    $proposalAdmissionDecisionFailures = @($reportStatus.self_improve_proposal_memory_admission_decision_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_decision_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_decision_source) targets=$($reportStatus.self_improve_proposal_memory_admission_decision_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_decision_request_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_decision_blocked_count) first_candidate=$($reportStatus.self_improve_proposal_memory_admission_decision_first_candidate) writer_required=$($reportStatus.self_improve_proposal_memory_admission_decision_writer_required) preflight_passed=$($reportStatus.self_improve_proposal_memory_admission_decision_admission_writer_preflight_passed) explicit_writer_invocation_required=$($reportStatus.self_improve_proposal_memory_admission_decision_explicit_writer_invocation_required) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_decision_admission_write_authorized) gate_blocked=$($reportStatus.self_improve_proposal_memory_admission_decision_gate_blocked) failure_reasons=$proposalAdmissionDecisionFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_decision_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_decision_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_decision_ndkv_write_allowed)"
    $proposalAdmissionWriterPlanFailures = @($reportStatus.self_improve_proposal_memory_admission_writer_plan_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_writer_plan_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_source) targets=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_request_count) plan_items=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_item_count) ready=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_ready_plan_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_first_plan_item) writer_plan_ready=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_ready) explicit_writer_invocation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_explicit_writer_invocation_required) experiment_required=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_experiment_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_rollback_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_validation_required) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_admission_write_authorized) failure_reasons=$proposalAdmissionWriterPlanFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_plan_ndkv_write_allowed)"
    $proposalAdmissionWriterDryRunFailures = @($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_writer_dry_run_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_source) targets=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_request_count) plan_items=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_plan_item_count) dry_run_items=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_item_count) ready=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_ready_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_first_item) dry_run_ready=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_ready) explicit_writer_invocation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_explicit_writer_invocation_required) dry_run_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_required) experiment_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_experiment_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_rollback_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_validation_required) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_admission_write_authorized) failure_reasons=$proposalAdmissionWriterDryRunFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_ndkv_write_allowed)"
    $proposalAdmissionWriterDryRunReceiptFailures = @($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_source) targets=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_request_count) dry_run_items=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_dry_run_item_count) receipt_items=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_item_count) succeeded=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_succeeded_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_first_item) dry_run_receipt_ready=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_ready) explicit_writer_invocation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_explicit_writer_invocation_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_commit_allowed) validation_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_rollback_required) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_admission_write_authorized) failure_reasons=$proposalAdmissionWriterDryRunReceiptFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_writer_dry_run_receipt_ndkv_write_allowed)"
    $proposalAdmissionCommitRecordStageFailures = @($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_commit_record_stage_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_source) targets=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_request_count) receipt_items=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_receipt_item_count) commit_record_items=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_item_count) staged=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_staged_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_first_item) commit_record_stage_ready=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_ready) explicit_writer_invocation_required=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_explicit_writer_invocation_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_admission_write_authorized) failure_reasons=$proposalAdmissionCommitRecordStageFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_record_stage_ndkv_write_allowed)"
    $proposalAdmissionCommitApprovalRequestFailures = @($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_commit_approval_request_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_source) targets=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_request_count) commit_record_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_commit_record_item_count) approval_request_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_item_count) requested=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_requested_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_first_item) commit_approval_request_ready=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_ready) explicit_commit_approval_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_explicit_commit_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_admission_write_authorized) failure_reasons=$proposalAdmissionCommitApprovalRequestFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_request_ndkv_write_allowed)"
    $proposalAdmissionCommitApprovalDecisionFailures = @($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_commit_approval_decision_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_source) targets=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_request_count) approval_request_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_request_item_count) approval_decision_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_item_count) recorded=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_recorded_count) approved=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_approved_count) pending=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_pending_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_first_item) commit_approval_decision_ready=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_ready) explicit_commit_approval_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_explicit_commit_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_admission_write_authorized) failure_reasons=$proposalAdmissionCommitApprovalDecisionFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_decision_ndkv_write_allowed)"
    $proposalAdmissionCommitApprovalReviewPacketFailures = @($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1: source=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_source) targets=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_target_count) requests=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_request_count) approval_request_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_request_item_count) approval_decision_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_decision_item_count) review_packet_items=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_item_count) ready=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_ready_count) pending=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_pending_count) blocked=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_blocked_count) first_item=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_first_item) approval_review_packet_ready=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_admission_write_authorized) failure_reasons=$proposalAdmissionCommitApprovalReviewPacketFailures auto_apply=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_admission_commit_approval_review_packet_ndkv_write_allowed)"
    $proposalMemoryReflectionUsefulnessFailures = @($reportStatus.self_improve_proposal_memory_reflection_usefulness_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_usefulness_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_target_count) projected=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_projected_report_count) accepted_memory=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_accepted_memory_admission_count) quarantined=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_quarantined_candidate_count) review_packet_items=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_review_packet_item_count) useful=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_useful_count) pending_operator_approval=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_pending_operator_approval_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_blocked_count) wasted_compute_guard=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_wasted_compute_guard_count) adapter_safe=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_adapter_safe_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_first_item) reflection_usefulness_ready=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_admission_write_authorized) failure_reasons=$proposalMemoryReflectionUsefulnessFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_usefulness_ndkv_write_allowed)"
    $proposalMemoryReflectionDedupeClusterFailures = @($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_dedupe_cluster_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_target_count) useful=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_useful_count) clusters=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_reflection_item_count) wasted_compute_guard=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_wasted_compute_guard_count) pending_operator_approval=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_pending_operator_approval_count) adapter_safe=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_adapter_safe_count) first_cluster=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_first_cluster) reflection_dedupe_ready=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_admission_write_authorized) failure_reasons=$proposalMemoryReflectionDedupeClusterFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_dedupe_cluster_ndkv_write_allowed)"
    $proposalMemoryReflectionReusePlanFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_plan_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_target_count) clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_cluster_count) plan_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_item_count) ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_ready_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_projected_saved_reflection_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_first_item) reflection_reuse_plan_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_admission_write_authorized) failure_reasons=$proposalMemoryReflectionReusePlanFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_plan_ndkv_write_allowed)"
    $proposalMemoryReflectionReusePreflightFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_preflight_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_target_count) plan_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_plan_item_count) ready_plan_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_ready_reuse_plan_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_item_count) passed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_passed_item_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_blocked_item_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_first_item) reuse_preflight_passed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_passed) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_execution_authorized) failure_reasons=$proposalMemoryReflectionReusePreflightFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_preflight_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupPreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_target_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_preflight_item_count) lookup_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_item_count) ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_ready_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_blocked_item_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_projected_model_call_skip_count) first_lookup_key=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_first_lookup_key) lookup_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupPreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_preview_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalRequestFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_target_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_preflight_item_count) lookup_preview_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_preview_item_count) ready_lookup_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_lookup_preview_count) approval_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_item_count) ready_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_count) requested=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_requested_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_first_item) lookup_approval_request_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalRequestFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalDecisionPreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_target_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_preflight_item_count) lookup_preview_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_preview_item_count) ready_lookup_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_lookup_preview_count) approval_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approval_request_item_count) ready_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_approval_request_count) decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_item_count) ready_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approved_lookup_execution_count) pending=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_pending_approval_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_first_item) lookup_approval_decision_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalDecisionPreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenIntakePreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_target_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_preflight_item_count) lookup_preview_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_preview_item_count) ready_lookup_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_lookup_preview_count) approval_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_request_item_count) ready_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_request_count) decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_decision_preview_item_count) ready_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_decision_preview_count) token_intake_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_item_count) ready_token_intake_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_pending_operator_token_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_first_item) lookup_approval_token_intake_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenIntakePreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_target_count) preflight_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_preflight_item_count) lookup_preview_items=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_preview_item_count) ready_lookup_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_lookup_preview_count) approval_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_request_item_count) ready_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_request_count) decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_decision_preview_item_count) ready_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_decision_preview_count) token_intake_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_token_intake_preview_item_count) ready_token_intake_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_token_intake_preview_count) token_intake_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_item_count) ready_token_intake_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_pending_operator_token_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approved_lookup_execution_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_first_item) lookup_approval_token_intake_decision_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenIntakeDecisionPreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_target_count) token_intake_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_token_intake_decision_preview_item_count) ready_token_intake_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_token_intake_decision_preview_count) token_decision_record_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_item_count) ready_token_decision_record_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_pending_operator_token_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_approved_lookup_execution_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_first_item) lookup_approval_token_decision_record_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordPreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequestFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_target_count) token_decision_record_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_token_decision_record_preview_item_count) ready_token_decision_record_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_token_decision_record_preview_count) token_decision_record_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_item_count) ready_token_decision_record_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_count) requested_token_decision_records=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_requested_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_pending_operator_token_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_approved_lookup_execution_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_first_item) lookup_approval_token_decision_record_request_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordRequestFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_target_count) token_decision_record_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_token_decision_record_request_item_count) ready_token_decision_record_requests=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_token_decision_record_request_count) token_decision_record_review_packets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_item_count) ready_token_decision_record_review_packets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_count) requested_token_decision_records=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_requested_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_pending_operator_token_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_approved_lookup_execution_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_first_item) lookup_approval_token_decision_record_review_packet_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ndkv_write_allowed)"
    $proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreviewFailures = @($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_source) targets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_target_count) token_decision_record_review_packets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_token_decision_record_review_packet_item_count) ready_token_decision_record_review_packets=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_token_decision_record_review_packet_count) review_packet_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_item_count) ready_review_packet_decision_previews=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_count) requested_token_decision_records=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_requested_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_pending_operator_token_count) approved_lookup_executions=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approved_lookup_execution_count) blocked=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rejection_token_present_count) duplicate_clusters=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_cluster_count) duplicate_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_reflection_item_count) projected_saved_reflections=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_saved_reflection_count) projected_model_call_skips=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_model_call_skip_count) first_item=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_first_item) lookup_approval_token_decision_record_review_packet_decision_preview_ready=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_admission_write_authorized) model_call_skip_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_model_call_skip_authorized) reflection_reuse_execution_authorized=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_execution_authorized) memory_lookup_performed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_lookup_performed) lookup_hit_assumed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_lookup_hit_assumed) read_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_read_only) report_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_only) candidate_only=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_candidate_only) failure_reasons=$proposalMemoryReflectionReuseLookupApprovalTokenDecisionRecordReviewPacketDecisionPreviewFailures auto_apply=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ndkv_write_allowed)"
    $proposalMemoryApprovalTokenIntakeFailures = @($reportStatus.self_improve_proposal_memory_approval_token_intake_failure_reasons | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join ","
    Write-Host "report_self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1: source=$($reportStatus.self_improve_proposal_memory_approval_token_intake_source) targets=$($reportStatus.self_improve_proposal_memory_approval_token_intake_target_count) review_packet_items=$($reportStatus.self_improve_proposal_memory_approval_token_intake_review_packet_item_count) useful_reflections=$($reportStatus.self_improve_proposal_memory_approval_token_intake_useful_reflection_item_count) intake_items=$($reportStatus.self_improve_proposal_memory_approval_token_intake_item_count) ready=$($reportStatus.self_improve_proposal_memory_approval_token_intake_ready_count) pending_operator_tokens=$($reportStatus.self_improve_proposal_memory_approval_token_intake_pending_operator_token_count) blocked=$($reportStatus.self_improve_proposal_memory_approval_token_intake_blocked_count) approval_tokens=$($reportStatus.self_improve_proposal_memory_approval_token_intake_approval_token_present_count) rejection_tokens=$($reportStatus.self_improve_proposal_memory_approval_token_intake_rejection_token_present_count) first_item=$($reportStatus.self_improve_proposal_memory_approval_token_intake_first_item) approval_token_intake_ready=$($reportStatus.self_improve_proposal_memory_approval_token_intake_ready) explicit_operator_approval_required=$($reportStatus.self_improve_proposal_memory_approval_token_intake_explicit_operator_approval_required) validation_required=$($reportStatus.self_improve_proposal_memory_approval_token_intake_validation_required) rollback_required=$($reportStatus.self_improve_proposal_memory_approval_token_intake_rollback_required) commit_allowed=$($reportStatus.self_improve_proposal_memory_approval_token_intake_commit_allowed) admission_write_authorized=$($reportStatus.self_improve_proposal_memory_approval_token_intake_admission_write_authorized) failure_reasons=$proposalMemoryApprovalTokenIntakeFailures auto_apply=$($reportStatus.self_improve_proposal_memory_approval_token_intake_auto_apply) memory_store_write_allowed=$($reportStatus.self_improve_proposal_memory_approval_token_intake_memory_store_write_allowed) ndkv_write_allowed=$($reportStatus.self_improve_proposal_memory_approval_token_intake_ndkv_write_allowed)"
}
Write-Host "next_round_decision: schema=$($nextRoundDecision.schema) display_state=$($nextRoundDecision.display_state) safe_to_wait_current_round_active=$($nextRoundDecision.safe_to_wait_current_round_active) safe_to_continue_after_current_round=$($nextRoundDecision.safe_to_continue_after_current_round) operator_attention_blocked=$($nextRoundDecision.operator_attention_blocked) reason=$($nextRoundDecision.reason_code) read_only=$($nextRoundDecision.read_only) report_only=$($nextRoundDecision.report_only) side_effects=$($nextRoundDecision.side_effects)"
Write-Host "next_round_decision_report_v1: schema=$($nextRoundDecisionReportV1.schema) display_state=$($nextRoundDecisionReportV1.display_state) safe_to_wait_current_round_active=$($nextRoundDecisionReportV1.safe_to_wait_current_round_active) safe_to_continue_after_current_round=$($nextRoundDecisionReportV1.safe_to_continue_after_current_round) operator_attention_blocked=$($nextRoundDecisionReportV1.operator_attention_blocked) reason=$($nextRoundDecisionReportV1.reason_code) read_only=$($nextRoundDecisionReportV1.read_only) report_only=$($nextRoundDecisionReportV1.report_only) side_effects=$($nextRoundDecisionReportV1.side_effects)"
Write-Host "next_round_downstream_status_consumers_v1: schema=$($nextRoundDownstreamStatusConsumersV1.schema) effective_decision_status=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.effective_decision_status) service_cli_display_status=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.service_cli_display_status) forge_operator_display_status=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.forge_operator_display_status) agent_assignment_acceptance=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.agent_assignment_acceptance) memory_self_improve_admission_visibility=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.memory_self_improve_admission_visibility) active_round=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.active_round) ledger_round=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.ledger_latest_round) latest_done_round=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.latest_done_round) round_id_source=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.round_id_evidence.source_schema) read_only=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.read_only) report_only=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.report_only) no_side_effects=$($nextRoundDownstreamStatusConsumersV1.next_round_downstream.no_side_effects)"
Write-Host "next_step: $($status.next_step)"
exit $StatusExitCode
