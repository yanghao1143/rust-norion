$ErrorActionPreference = "Stop"

$docPath = Join-Path $PSScriptRoot "lane-handoff-protocol.md"
if (!(Test-Path -LiteralPath $docPath)) {
    throw "Missing lane handoff protocol: $docPath"
}

$doc = Get-Content -Raw -LiteralPath $docPath
$required = @(
    "lane_manifest:",
    "owner_window_id:",
    "source_issue:",
    "allowed_paths:",
    "forbidden_paths:",
    "validation_commands:",
    "handoff_packet:",
    "touched_files:",
    "commands_run:",
    "unresolved_risks:",
    "approval_required: true",
    "CrossWindowExperiencePacket",
    "AgentHandoffSanitizer",
    "duplicate_packet",
    "file_overlap",
    "lane_owner_collision",
    "stale_packet",
    "polluted_payload",
    "budget_exceeded"
)

foreach ($needle in $required) {
    if (!$doc.Contains($needle)) {
        throw "Missing required lane handoff token: $needle"
    }
}

if ($doc -match "password=|sk-[A-Za-z0-9]|BEGIN PRIVATE|raw prompt password") {
    throw "Protocol doc contains a forbidden secret or raw-payload example"
}

"lane_handoff_protocol_check=PASS"
