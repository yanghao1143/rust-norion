param(
    [string]$Prompt = "Say hello in Chinese and briefly explain the rust-norion Gemma lab status.",
    [string]$Lab = "http://127.0.0.1:8787",
    [ValidateSet("raw", "enhanced")]
    [string]$Output = "raw",
    [ValidateSet("chat", "generate", "business-cycle")]
    [string]$Endpoint = "chat",
    [string]$Profile = "coding",
    [int]$MaxTokens = 262144,
    [double]$FeedbackAmount = 0.5,
    [switch]$NoSelfImprove,
    [string]$RustCheckCode = "",
    [int]$TimeoutSeconds = 900,
    [switch]$ShowMeta,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

trap {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}

if ($Help) {
    Write-Host "Stream one prompt through a running rustgpt-lab Web Lab without starting Gemma."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt `"hello`""
    Write-Host "  .\tools\rustgpt-lab\chat-gemma-lab.cmd -Endpoint business-cycle -Prompt `"check integration`" -ShowMeta"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Lab <url>              rustgpt-lab URL, default http://127.0.0.1:8787"
    Write-Host "  -Endpoint <mode>        chat, generate, or business-cycle"
    Write-Host "  -Output <mode>          raw or enhanced answer view, default raw"
    Write-Host "  -Profile <name>         request profile sent to rust-norion, default coding"
    Write-Host "  -TimeoutSeconds <n>     total PowerShell SSE client wait window, default 900; not a socket read poll"
    Write-Host "  -MaxTokens <n>          generation budget sent as max_tokens, default 262144; not context message count"
    Write-Host "  -FeedbackAmount <n>     business-cycle feedback amount, default 0.5"
    Write-Host "  -NoSelfImprove          send self_improve=false for business-cycle"
    Write-Host "  -RustCheckCode <code>   optional Rust code sent to business-cycle checks"
    Write-Host "  -ShowMeta               print meta/final JSON events"
    Write-Host ""
    Write-Host "The script only connects to an already running Web Lab. It exits nonzero on SSE error, timeout, EOF before done, or incomplete SSE frames."
    return
}

function Get-LabBaseUrl {
    param([string]$Url)
    $Url.TrimEnd("/")
}

function Convert-SseFieldValue {
    param([string]$Value)

    if ($Value.StartsWith(" ")) {
        return $Value.Substring(1)
    }
    return $Value
}

function Throw-LabChatStreamReadFailure {
    param(
        [System.Exception]$Error,
        [int]$TimeoutSeconds,
        [object]$StreamError,
        [bool]$HasPendingFrame
    )

    $baseError = $Error
    if ($baseError -is [System.AggregateException]) {
        $baseError = $baseError.GetBaseException()
    }
    if ($baseError -is [System.OperationCanceledException] -or
        $baseError -is [System.Threading.Tasks.TaskCanceledException]) {
        throw "lab chat stream timed out after ${TimeoutSeconds}s"
    }
    if ($null -ne $StreamError) {
        $streamErrorText = [string]$StreamError
        if ([string]::IsNullOrWhiteSpace($streamErrorText)) {
            throw "lab chat stream returned SSE error"
        }
        throw "lab chat stream returned SSE error: $streamErrorText"
    }
    if ($HasPendingFrame) {
        throw "lab chat stream truncated: incomplete SSE frame before EOF"
    }
    throw "lab chat stream truncated: EOF before done event"
}

function Read-LabHealth {
    param([string]$LabBase)
    try {
        Invoke-RestMethod -Uri "$LabBase/api/backend-health" -TimeoutSec 4
    } catch {
        throw "rustgpt-lab Web Lab is not reachable at $LabBase (default 8787). Start the safe built-in UI with tools\rustgpt-lab\start-built-in-lab.cmd, attach Web Lab to an existing 7878 backend with cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- --backend 127.0.0.1:7878 --bind 127.0.0.1:8787, or inspect with tools\rustgpt-lab\status-built-in-lab.cmd and tools\rustgpt-lab\status-gemma-lab.cmd. Do not send prompts to 8686; it is only the optional Gemma runtime behind rust-norion."
    }
}

function Test-PositiveNumber {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    $number = 0.0
    if (-not [double]::TryParse([string]$Value, [ref]$number)) {
        return $false
    }
    return $number -gt 0
}

function Join-LabFailures {
    param([object]$Failures)

    if ($null -eq $Failures) {
        return ""
    }
    $items = @($Failures) | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if ($items.Count -eq 0) {
        return ""
    }
    return ": " + ($items -join "; ")
}

function Assert-ExperiencePromptGate {
    param([object]$Hygiene)

    if ($null -eq $Hygiene) {
        return
    }
    if ($Hygiene.clean -eq $false) {
        throw "rust-norion backend prompt gate failed: experience_hygiene.clean=false"
    }
    if (Test-PositiveNumber $Hygiene.quarantine_candidates) {
        throw "rust-norion backend prompt gate failed: experience quarantine_candidates=$($Hygiene.quarantine_candidates)"
    }
    if (Test-PositiveNumber $Hygiene.repairable_legacy_metadata_lessons) {
        throw "rust-norion backend prompt gate failed: experience repairable_legacy_metadata_lessons=$($Hygiene.repairable_legacy_metadata_lessons)"
    }
    if (Test-PositiveNumber $Hygiene.repairable_index_records) {
        throw "rust-norion backend prompt gate failed: experience repairable_index_records=$($Hygiene.repairable_index_records)"
    }
    if ($null -ne $Hygiene.index) {
        if ($Hygiene.index.retrieval_ready -eq $false) {
            throw "rust-norion backend prompt gate failed: experience index retrieval_ready=false"
        }
        if ([string]$Hygiene.index.risk_level -eq "blocked") {
            throw "rust-norion backend prompt gate failed: experience index risk_level=blocked"
        }
    }
}

function Assert-LabReady {
    param([object]$Health)

    if (-not $Health.ok) {
        $message = if ($Health.error) { $Health.error } else { "backend health returned ok=false" }
        throw "rust-norion backend is not ready: $message"
    }

    if ($Health.engine_busy) {
        throw "rust-norion backend is busy. Wait for the active Gemma request to finish, then rerun this script."
    }

    if ($Health.readiness_ok -eq $false) {
        throw "rust-norion backend prompt gate failed: readiness=false$(Join-LabFailures $Health.readiness_failures)"
    }

    if ($Health.safe_device_ok -eq $false) {
        throw "rust-norion backend prompt gate failed: safe-device=false$(Join-LabFailures $Health.safe_device_failures)"
    }

    Assert-ExperiencePromptGate -Hygiene $Health.experience_hygiene

    if ($Health.gemma_runtime_server -and $Health.gemma_runtime_reachable -eq $false) {
        throw "Gemma runtime is configured but not reachable at $($Health.gemma_runtime_server). Inspect with tools\rustgpt-lab\status-gemma-lab.cmd and run tools\rustgpt-lab\start-gemma-lab.cmd -CheckOnly before any real start; only run start-gemma-lab.cmd without -CheckOnly when you intentionally want to start the Gemma stack. Do not send prompts to 8686 directly."
    }
}

function Write-SseEvent {
    param(
        [string]$Event,
        [string]$Data,
        [switch]$ShowMeta
    )

    switch ($Event) {
        "delta" {
            Write-Host -NoNewline $Data
        }
        "stage" {
            Write-Host ""
            Write-Host "[stage] $Data" -ForegroundColor Cyan
        }
        "status" {
            Write-Host ""
            Write-Host "[status] $Data" -ForegroundColor DarkGray
        }
        "heartbeat" {
            Write-Host ""
            Write-Host "[heartbeat] $Data" -ForegroundColor DarkGray
        }
        "done" {
            Write-Host ""
            Write-Host "[DONE]"
        }
        "error" {
            Write-Host ""
            Write-Host "[error] $Data" -ForegroundColor Red
        }
        "meta" {
            if ($ShowMeta) {
                Write-Host ""
                Write-Host "[meta] $Data" -ForegroundColor DarkGray
            }
        }
        "raw" {}
        "enhanced" {}
        "final" {
            Write-FinalEvent -Data $Data -ShowMeta:$ShowMeta
        }
        default {
            Write-Host ""
            Write-Host "[$Event] $Data" -ForegroundColor DarkGray
        }
    }
}

function Write-FinalEvent {
    param(
        [string]$Data,
        [switch]$ShowMeta
    )

    try {
        $json = $Data | ConvertFrom-Json -ErrorAction Stop
    } catch {
        Write-Host ""
        Write-Host "[final] $Data" -ForegroundColor DarkGray
        return
    }

    $answer = $json.answer
    if (-not $answer -and $json.generate) {
        $answer = $json.generate.answer
    }
    $elapsed = $json.elapsed_ms
    if (-not $elapsed -and $json.generate) {
        $elapsed = $json.generate.elapsed_ms
    }
    $tokens = $json.runtime_token_count
    if (-not $tokens -and $json.generate) {
        $tokens = $json.generate.runtime_token_count
    }

    Write-Host ""
    if ($json.business_cycle) {
        Write-Host ("[final] business_cycle passed={0} elapsed_ms={1} runtime_tokens={2}" -f $json.business_cycle.passed, $elapsed, $tokens) -ForegroundColor Green
        Write-Host ("[final] feedback_applied={0} rust_check_passed={1} self_improve_passed={2}" -f $json.business_cycle.feedback_applied, $json.business_cycle.rust_check_passed, $json.business_cycle.self_improve_passed) -ForegroundColor Green
    } else {
        Write-Host ("[final] elapsed_ms={0} runtime_tokens={1}" -f $elapsed, $tokens) -ForegroundColor Green
    }

    if ($answer) {
        Write-Host ""
        Write-Host "[final answer]"
        Write-Host $answer
    }

    if ($ShowMeta) {
        Write-Host ""
        Write-Host "[final json]" -ForegroundColor DarkGray
        Write-Host $Data -ForegroundColor DarkGray
    }
}

function Invoke-LabChatStream {
    param(
        [string]$LabBase,
        [string]$Body,
        [int]$TimeoutSeconds,
        [switch]$ShowMeta
    )

    $streamTimeoutSeconds = [Math]::Max(1, $TimeoutSeconds)
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($streamTimeoutSeconds)
    Add-Type -AssemblyName System.Net.Http
    $client = [System.Net.Http.HttpClient]::new()
    $client.Timeout = [TimeSpan]::FromSeconds($streamTimeoutSeconds)
    $request = [System.Net.Http.HttpRequestMessage]::new(
        [System.Net.Http.HttpMethod]::Post,
        "$LabBase/api/chat-stream"
    )
    $response = $null
    $reader = $null
    try {
        $request.Content = [System.Net.Http.StringContent]::new(
            $Body,
            [System.Text.Encoding]::UTF8,
            "application/json"
        )

        try {
            $response = $client.SendAsync(
                $request,
                [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead
            ).GetAwaiter().GetResult()
        } catch [System.OperationCanceledException] {
            throw "lab chat stream timed out after ${streamTimeoutSeconds}s"
        }

        if (-not $response.IsSuccessStatusCode) {
            $errorBody = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
            throw "chat stream request failed with HTTP $([int]$response.StatusCode): $errorBody"
        }

        $stream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8)
        $event = "message"
        $data = [System.Collections.Generic.List[string]]::new()
        $sawDone = $false
        $streamError = $null

        while ($true) {
            $remainingMs = [int][Math]::Ceiling(($deadline - [DateTimeOffset]::UtcNow).TotalMilliseconds)
            if ($remainingMs -le 0) {
                throw "lab chat stream timed out after ${streamTimeoutSeconds}s"
            }
            $lineTask = $null
            $completed = $false
            $line = $null
            try {
                $lineTask = $reader.ReadLineAsync()
                $completed = $lineTask.Wait($remainingMs)
                if ($completed) {
                    $line = $lineTask.GetAwaiter().GetResult()
                }
            } catch {
                Throw-LabChatStreamReadFailure `
                    -Error $_.Exception `
                    -TimeoutSeconds $streamTimeoutSeconds `
                    -StreamError $streamError `
                    -HasPendingFrame ($data.Count -gt 0 -or $event -ne "message")
            }
            if (-not $completed) {
                throw "lab chat stream timed out after ${streamTimeoutSeconds}s"
            }
            if ($null -eq $line) {
                if ($data.Count -gt 0 -or $event -ne "message") {
                    throw "lab chat stream truncated: incomplete SSE frame before EOF"
                }
                break
            }

            if ($line.Length -eq 0) {
                $currentEvent = $event
                $currentData = $data.ToArray() -join "`n"
                if ($currentEvent.Length -eq 0) {
                    $currentEvent = "message"
                }
                if ($data.Count -gt 0 -or $currentEvent -ne "message") {
                    Write-SseEvent -Event $currentEvent -Data $currentData -ShowMeta:$ShowMeta
                }
                if ($currentEvent -eq "error" -and $null -eq $streamError) {
                    $streamError = $currentData
                }
                $event = "message"
                $data.Clear()
                if ($currentEvent -eq "done") {
                    $sawDone = $true
                    break
                }
                continue
            }

            if ($line.StartsWith("event:")) {
                $event = Convert-SseFieldValue -Value $line.Substring(6)
            } elseif ($line -eq "event") {
                $event = ""
            } elseif ($line.StartsWith("data:")) {
                $data.Add((Convert-SseFieldValue -Value $line.Substring(5)))
            } elseif ($line -eq "data") {
                $data.Add("")
            }
        }

        if ($null -ne $streamError) {
            if ([string]::IsNullOrWhiteSpace($streamError)) {
                throw "lab chat stream returned SSE error"
            }
            throw "lab chat stream returned SSE error: $streamError"
        }
        if (-not $sawDone) {
            throw "lab chat stream truncated: EOF before done event"
        }
    } finally {
        if ($null -ne $reader) {
            $reader.Dispose()
        }
        if ($null -ne $response) {
            $response.Dispose()
        }
        $request.Dispose()
        $client.Dispose()
    }
}

$labBase = Get-LabBaseUrl -Url $Lab
$health = Read-LabHealth -LabBase $labBase
Assert-LabReady -Health $health

$payload = [ordered]@{
    prompt = $Prompt
    profile = $Profile
    output = $Output
    endpoint = $Endpoint
    max_tokens = $MaxTokens
    feedback_amount = $FeedbackAmount
    self_improve = (-not $NoSelfImprove)
    rust_check_code = $RustCheckCode
}

$body = $payload | ConvertTo-Json -Compress
Write-Host "Streaming from $labBase/api/chat-stream"
Write-Host "endpoint=$Endpoint output=$Output profile=$Profile max_tokens=$MaxTokens timeout_seconds=$TimeoutSeconds"
Write-Host ""
Invoke-LabChatStream -LabBase $labBase -Body $body -TimeoutSeconds $TimeoutSeconds -ShowMeta:$ShowMeta
