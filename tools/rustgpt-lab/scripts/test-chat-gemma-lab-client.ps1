param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate rustgpt-lab wrapper safety and chat SSE client behavior offline."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\test-gemma-lab-safety.cmd"
    Write-Host "  .\tools\rustgpt-lab\test-chat-gemma-lab-client.cmd"
    Write-Host ""
    Write-Host "The older test-chat-gemma-lab-client.cmd name remains as a compatibility alias."
    Write-Host "The Web UI parser and interaction checks require Node.js on PATH."
    Write-Host "This helper does not SSH, start or stop Gemma, or send real inference prompts."
    Write-Host ""
    Write-Host "Checks:"
    Write-Host "  - chat/repl/start/status/stop wrapper help text is reachable"
    Write-Host "  - Gemma and built-in wrapper help locks the 7878 backend, 8787 Web Lab, and 8686 runtime port map"
    Write-Host "  - Gemma and built-in start wrappers support read-only -CheckOnly"
    Write-Host "  - Gemma and built-in status/stop wrappers stay read-only or dry-run on random ports"
    Write-Host "  - success stream prints delta, final, and done"
    Write-Host "  - heartbeat stream prints progress and still completes"
    Write-Host "  - comment-only SSE keep-alive frames are ignored"
    Write-Host "  - CR-only SSE frame separators are parsed"
    Write-Host "  - multiline SSE data fields are joined with newlines"
    Write-Host "  - empty SSE event fields fall back to message"
    Write-Host "  - no-colon SSE fields are parsed like the Rust/Web clients"
    Write-Host "  - SSE field values remove only one optional leading space"
    Write-Host "  - PowerShell chat client sends business-cycle endpoint, output, profile, Rust check, self-improve, and feedback payload fields"
    Write-Host "  - SSE error exits nonzero with or without a trailing done"
    Write-Host "  - HTTP stream setup failures exit nonzero with the status and body"
    Write-Host "  - EOF before done, including after final, or inside a frame exits nonzero as truncated"
    Write-Host "  - Web UI SSE parser handles core frame edges and retains incomplete frames"
    Write-Host "  - Web UI Enter/Shift+Enter, send/input state, context-window clamp, cancel/error/truncated draft restore, heartbeat progress, auto-scroll, clear-context including mid-stream clears, busy/readiness/safe-device/experience preflight gate blocks, HTTP failure recovery, rejected low-window history preservation, and rejected-stream context safety are covered offline"
    Write-Host "  - REPL -SkipStart stays attach-only: a missing backend exits before any Gemma/start/REPL path"
    Write-Host "  - PowerShell chat client stops before /api/chat-stream when Web Lab is unreachable or backend busy/readiness/safe-device/experience/Gemma runtime preflight fails"
    Write-Host "  - idle or pre-header streams exit nonzero on -TimeoutSeconds"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$clientScript = Join-Path $RepoRoot "tools\rustgpt-lab\scripts\chat-gemma-lab.ps1"
if (-not (Test-Path -LiteralPath $clientScript -PathType Leaf)) {
    throw "chat-gemma-lab.ps1 not found: $clientScript"
}
$clientCmd = Join-Path $RepoRoot "tools\rustgpt-lab\chat-gemma-lab.cmd"
if (-not (Test-Path -LiteralPath $clientCmd -PathType Leaf)) {
    throw "chat-gemma-lab.cmd not found: $clientCmd"
}
$webAppScript = Join-Path $RepoRoot "tools\rustgpt-lab\web\app.js"
if (-not (Test-Path -LiteralPath $webAppScript -PathType Leaf)) {
    throw "web app.js not found: $webAppScript"
}
$safetyCmd = Join-Path $RepoRoot "tools\rustgpt-lab\test-gemma-lab-safety.cmd"
if (-not (Test-Path -LiteralPath $safetyCmd -PathType Leaf)) {
    throw "test-gemma-lab-safety.cmd not found: $safetyCmd"
}
$legacySafetyCmd = Join-Path $RepoRoot "tools\rustgpt-lab\test-chat-gemma-lab-client.cmd"
if (-not (Test-Path -LiteralPath $legacySafetyCmd -PathType Leaf)) {
    throw "test-chat-gemma-lab-client.cmd not found: $legacySafetyCmd"
}
$replCmd = Join-Path $RepoRoot "tools\rustgpt-lab\repl-gemma-lab.cmd"
if (-not (Test-Path -LiteralPath $replCmd -PathType Leaf)) {
    throw "repl-gemma-lab.cmd not found: $replCmd"
}
$startCmd = Join-Path $RepoRoot "tools\rustgpt-lab\start-gemma-lab.cmd"
if (-not (Test-Path -LiteralPath $startCmd -PathType Leaf)) {
    throw "start-gemma-lab.cmd not found: $startCmd"
}
$statusCmd = Join-Path $RepoRoot "tools\rustgpt-lab\status-gemma-lab.cmd"
if (-not (Test-Path -LiteralPath $statusCmd -PathType Leaf)) {
    throw "status-gemma-lab.cmd not found: $statusCmd"
}
$stopCmd = Join-Path $RepoRoot "tools\rustgpt-lab\stop-gemma-lab.cmd"
if (-not (Test-Path -LiteralPath $stopCmd -PathType Leaf)) {
    throw "stop-gemma-lab.cmd not found: $stopCmd"
}
$startBuiltInCmd = Join-Path $RepoRoot "tools\rustgpt-lab\start-built-in-lab.cmd"
if (-not (Test-Path -LiteralPath $startBuiltInCmd -PathType Leaf)) {
    throw "start-built-in-lab.cmd not found: $startBuiltInCmd"
}
$statusBuiltInCmd = Join-Path $RepoRoot "tools\rustgpt-lab\status-built-in-lab.cmd"
if (-not (Test-Path -LiteralPath $statusBuiltInCmd -PathType Leaf)) {
    throw "status-built-in-lab.cmd not found: $statusBuiltInCmd"
}
$stopBuiltInCmd = Join-Path $RepoRoot "tools\rustgpt-lab\stop-built-in-lab.cmd"
if (-not (Test-Path -LiteralPath $stopBuiltInCmd -PathType Leaf)) {
    throw "stop-built-in-lab.cmd not found: $stopBuiltInCmd"
}

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        return $listener.LocalEndpoint.Port
    } finally {
        $listener.Stop()
    }
}

function Start-FakeLab {
    param(
        [int]$Port,
        [ValidateSet("success", "heartbeat", "comment_only", "cr_only_frames", "multiline_data", "empty_event_field", "no_colon_fields", "field_value_spacing", "business_cycle_payload", "error", "error_without_done", "http_error", "truncated", "final_without_done", "incomplete_frame", "timeout", "no_headers_timeout")]
        [string]$Mode
    )

    Start-Job -ScriptBlock {
        param([int]$Port, [string]$Mode)

        $ErrorActionPreference = "Stop"
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
        $listener.Start()

        function Read-HttpRequest {
            param([System.Net.Sockets.NetworkStream]$Stream)

            $Stream.ReadTimeout = 5000
            $buffer = New-Object byte[] 4096
            $bytes = New-Object System.Collections.Generic.List[byte]
            $headerEnd = -1
            while ($true) {
                $read = $Stream.Read($buffer, 0, $buffer.Length)
                if ($read -le 0) {
                    break
                }
                for ($i = 0; $i -lt $read; $i++) {
                    $bytes.Add($buffer[$i])
                }
                $text = [System.Text.Encoding]::ASCII.GetString($bytes.ToArray())
                $headerEnd = $text.IndexOf("`r`n`r`n")
                $separatorLength = 4
                if ($headerEnd -lt 0) {
                    $headerEnd = $text.IndexOf("`n`n")
                    $separatorLength = 2
                }
                if ($headerEnd -ge 0) {
                    $headers = $text.Substring(0, $headerEnd)
                    $contentLength = 0
                    foreach ($line in ($headers -split "`r?`n")) {
                        if ($line -match '^content-length:\s*(\d+)\s*$') {
                            $contentLength = [int]$Matches[1]
                            break
                        }
                    }
                    $targetBytes = $headerEnd + $separatorLength + $contentLength
                    while ($bytes.Count -lt $targetBytes) {
                        $read = $Stream.Read($buffer, 0, $buffer.Length)
                        if ($read -le 0) {
                            break
                        }
                        for ($i = 0; $i -lt $read; $i++) {
                            $bytes.Add($buffer[$i])
                        }
                    }
                    return [System.Text.Encoding]::UTF8.GetString($bytes.ToArray())
                }
            }
            return [System.Text.Encoding]::UTF8.GetString($bytes.ToArray())
        }

        function Write-Ascii {
            param(
                [System.Net.Sockets.NetworkStream]$Stream,
                [string]$Text
            )

            $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
            $Stream.Write($bytes, 0, $bytes.Length)
            $Stream.Flush()
        }

        try {
            while ($true) {
                $client = $listener.AcceptTcpClient()
                try {
                    $stream = $client.GetStream()
                    $request = Read-HttpRequest -Stream $stream
                    $requestLine = ($request -split "`r?`n", 2)[0]

                    if ($requestLine -like "GET /api/backend-health *") {
                        $body = '{"ok":true,"engine_busy":false,"gemma_runtime_reachable":true}'
                        Write-Ascii -Stream $stream -Text "HTTP/1.1 200 OK`r`ncontent-type: application/json`r`ncontent-length: $($body.Length)`r`nconnection: close`r`n`r`n$body"
                        continue
                    }

                    if ($requestLine -like "POST /api/chat-stream *") {
                        if ($Mode -eq "no_headers_timeout") {
                            Start-Sleep -Seconds 10
                            break
                        }
                        if ($Mode -eq "http_error") {
                            $body = "fake upstream unavailable"
                            Write-Ascii -Stream $stream -Text "HTTP/1.1 503 Service Unavailable`r`ncontent-type: text/plain`r`ncontent-length: $($body.Length)`r`nconnection: close`r`n`r`n$body"
                            break
                        }
                        Write-Ascii -Stream $stream -Text "HTTP/1.1 200 OK`r`ncontent-type: text/event-stream`r`nconnection: close`r`n`r`n"
                        switch ($Mode) {
                            "success" {
                                Write-Ascii -Stream $stream -Text "event: delta`ndata: hello from fake lab`n`n"
                                Write-Ascii -Stream $stream -Text "event: final`ndata: {`"answer`":`"fake final`",`"elapsed_ms`":1,`"runtime_token_count`":2}`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "heartbeat" {
                                Write-Ascii -Stream $stream -Text "event: heartbeat`ndata: waiting on fake backend`n`n"
                                Write-Ascii -Stream $stream -Text "event: delta`ndata: after heartbeat`n`n"
                                Write-Ascii -Stream $stream -Text "event: final`ndata: {`"answer`":`"heartbeat final`",`"elapsed_ms`":2,`"runtime_token_count`":3}`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "comment_only" {
                                Write-Ascii -Stream $stream -Text ": keep-alive`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "cr_only_frames" {
                                Write-Ascii -Stream $stream -Text "event: delta`rdata: cr only`r`r"
                                Start-Sleep -Milliseconds 50
                                Write-Ascii -Stream $stream -Text "event: done`rdata: [DONE]`r`r`r"
                            }
                            "multiline_data" {
                                Write-Ascii -Stream $stream -Text "event: delta`ndata: line one`ndata: line two`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "empty_event_field" {
                                Write-Ascii -Stream $stream -Text "event:`ndata: empty event became message`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "no_colon_fields" {
                                Write-Ascii -Stream $stream -Text "event`ndata: no-colon event became message`n`n"
                                Write-Ascii -Stream $stream -Text "event: status`ndata`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "field_value_spacing" {
                                Write-Ascii -Stream $stream -Text "event: delta`ndata:   indented`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "business_cycle_payload" {
                                $bodyStart = $request.IndexOf("`r`n`r`n")
                                $separatorLength = 4
                                if ($bodyStart -lt 0) {
                                    $bodyStart = $request.IndexOf("`n`n")
                                    $separatorLength = 2
                                }
                                $requestBody = if ($bodyStart -ge 0) { $request.Substring($bodyStart + $separatorLength) } else { "" }
                                try {
                                    $payload = $requestBody | ConvertFrom-Json -ErrorAction Stop
                                } catch {
                                    $payload = $null
                                }
                                $feedback = 0.0
                                [void][double]::TryParse([string]$payload.feedback_amount, [ref]$feedback)
                                $mismatches = @()
                                if ($null -eq $payload) {
                                    $mismatches += "invalid-json"
                                } else {
                                    if ($payload.endpoint -ne "business-cycle") { $mismatches += "endpoint=$($payload.endpoint)" }
                                    if ($payload.output -ne "enhanced") { $mismatches += "output=$($payload.output)" }
                                    if ($payload.profile -ne "review") { $mismatches += "profile=$($payload.profile)" }
                                    if ($payload.self_improve -ne $false) { $mismatches += "self_improve=$($payload.self_improve)" }
                                    if ($payload.rust_check_code -ne "pub fn ok() -> bool { true }") { $mismatches += "rust_check_code=$($payload.rust_check_code)" }
                                    if ([Math]::Abs($feedback - 0.75) -gt 0.0001) { $mismatches += "feedback_amount=$($payload.feedback_amount)" }
                                }
                                if ($mismatches.Count -gt 0) {
                                    Write-Ascii -Stream $stream -Text "event: error`ndata: business-cycle payload mismatch: $($mismatches -join ', ')`n`nevent: done`ndata: [DONE]`n`n"
                                    break
                                }
                                Write-Ascii -Stream $stream -Text "event: stage`ndata: business-cycle payload accepted`n`n"
                                Write-Ascii -Stream $stream -Text "event: final`ndata: {`"answer`":`"business payload ok`",`"elapsed_ms`":4,`"runtime_token_count`":5,`"business_cycle`":{`"passed`":true,`"feedback_applied`":true,`"rust_check_passed`":true,`"self_improve_passed`":false}}`n`n"
                                Write-Ascii -Stream $stream -Text "event: done`ndata: [DONE]`n`n"
                            }
                            "error" {
                                Write-Ascii -Stream $stream -Text "event: status`ndata: checking gate`n`nevent: error`ndata: blocked by fake gate`n`nevent: done`ndata: [DONE]`n`n"
                            }
                            "error_without_done" {
                                Write-Ascii -Stream $stream -Text "event: status`ndata: checking gate`n`nevent: error`ndata: backend closed after error`n`n"
                            }
                            "truncated" {
                                Write-Ascii -Stream $stream -Text "event: delta`ndata: partial answer`n`n"
                                Start-Sleep -Milliseconds 200
                            }
                            "final_without_done" {
                                Write-Ascii -Stream $stream -Text "event: final`ndata: {`"answer`":`"final before eof`",`"elapsed_ms`":3,`"runtime_token_count`":4}`n`n"
                                Start-Sleep -Milliseconds 200
                            }
                            "incomplete_frame" {
                                Write-Ascii -Stream $stream -Text "event: delta`ndata: partial frame without separator"
                                Start-Sleep -Milliseconds 200
                            }
                            "timeout" {
                                Write-Ascii -Stream $stream -Text "event: status`ndata: waiting forever`n`n"
                                Start-Sleep -Seconds 10
                            }
                        }
                        Start-Sleep -Milliseconds 100
                        break
                    }

                    $body = "missing"
                    Write-Ascii -Stream $stream -Text "HTTP/1.1 404 Not Found`r`ncontent-type: text/plain`r`ncontent-length: $($body.Length)`r`nconnection: close`r`n`r`n$body"
                } finally {
                    $client.Close()
                }
            }
        } finally {
            $listener.Stop()
        }
    } -ArgumentList $Port, $Mode
}

function Wait-FakeLab {
    param([int]$Port)

    $deadline = (Get-Date).AddSeconds(5)
    do {
        try {
            $client = [System.Net.Sockets.TcpClient]::new()
            $async = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
            if ($async.AsyncWaitHandle.WaitOne(100)) {
                $client.EndConnect($async)
                $client.Close()
                return
            }
            $client.Close()
        } catch {
        }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)

    throw "fake lab did not listen on 127.0.0.1:$Port"
}

function Start-FakeHealthOnlyLab {
    param(
        [int]$Port,
        [string]$HealthBody
    )

    Start-Job -ScriptBlock {
        param([int]$Port, [string]$HealthBody)

        $ErrorActionPreference = "Stop"
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
        $listener.Start()

        function Write-Ascii {
            param(
                [System.Net.Sockets.NetworkStream]$Stream,
                [string]$Text
            )

            $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
            $Stream.Write($bytes, 0, $bytes.Length)
            $Stream.Flush()
        }

        try {
            $deadline = (Get-Date).AddSeconds(8)
            while ((Get-Date) -lt $deadline) {
                $client = $listener.AcceptTcpClient()
                try {
                    $stream = $client.GetStream()
                    $stream.ReadTimeout = 5000
                    $buffer = New-Object byte[] 4096
                    $read = $stream.Read($buffer, 0, $buffer.Length)
                    if ($read -le 0) {
                        continue
                    }
                    $request = [System.Text.Encoding]::ASCII.GetString($buffer, 0, $read)
                    $requestLine = ($request -split "`r?`n", 2)[0]
                    if ($requestLine -like "GET /api/backend-health *") {
                        Write-Ascii -Stream $stream -Text "HTTP/1.1 200 OK`r`ncontent-type: application/json`r`ncontent-length: $($HealthBody.Length)`r`nconnection: close`r`n`r`n$HealthBody"
                        break
                    }
                    if ($requestLine -like "POST /api/chat-stream *") {
                        $body = "unexpected prompt forwarded after blocked health"
                        Write-Ascii -Stream $stream -Text "HTTP/1.1 500 Internal Server Error`r`ncontent-type: text/plain`r`ncontent-length: $($body.Length)`r`nconnection: close`r`n`r`n$body"
                        break
                    }
                    $body = "missing"
                    Write-Ascii -Stream $stream -Text "HTTP/1.1 404 Not Found`r`ncontent-type: text/plain`r`ncontent-length: $($body.Length)`r`nconnection: close`r`n`r`n$body"
                } finally {
                    $client.Close()
                }
            }
        } finally {
            $listener.Stop()
        }
    } -ArgumentList $Port, $HealthBody
}

function Invoke-ClientPreflightBlockedCase {
    param(
        [string]$Name,
        [string]$HealthBody,
        [string[]]$MustContain,
        [string[]]$MustNotContain = @()
    )

    Write-Host ""
    Write-Host "safety_case=$Name"
    $port = Get-FreeTcpPort
    $job = Start-FakeHealthOnlyLab -Port $port -HealthBody $HealthBody
    try {
        Wait-FakeLab -Port $port
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & cmd.exe /c "`"$clientCmd`" -Lab `"http://127.0.0.1:$port`" -Prompt `"fake`" -TimeoutSeconds 5 -ShowMeta" 2>&1
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }

        $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            Write-Host $text.TrimEnd()
        }
        if ($exitCode -eq 0) {
            throw "case '$Name' expected failure but succeeded"
        }
        foreach ($needle in $MustContain) {
            if (-not $text.Contains($needle)) {
                throw "case '$Name' missing expected output: $needle"
            }
        }
        foreach ($needle in @("Streaming from", "/api/chat-stream", "At $clientScript") + $MustNotContain) {
            if ($text.Contains($needle)) {
                throw "case '$Name' included forbidden output: $needle"
            }
        }
    } finally {
        Stop-Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job $job -Force -ErrorAction SilentlyContinue | Out-Null
    }
}

function Invoke-ClientLabUnreachableCase {
    Write-Host ""
    Write-Host "safety_case=lab_unreachable"
    $port = Get-FreeTcpPort
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & cmd.exe /c "`"$clientCmd`" -Lab `"http://127.0.0.1:$port`" -Prompt `"fake`" -TimeoutSeconds 5 -ShowMeta" 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -eq 0) {
        throw "lab unreachable case expected failure but succeeded"
    }
    foreach ($needle in @(
        "rustgpt-lab Web Lab is not reachable",
        "default 8787",
        "start-built-in-lab.cmd",
        "--backend 127.0.0.1:7878 --bind 127.0.0.1:8787",
        "status-gemma-lab.cmd",
        "Do not send prompts to 8686"
    )) {
        if (-not $text.Contains($needle)) {
            throw "lab unreachable case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Streaming from",
        "/api/chat-stream",
        "Start it with tools\rustgpt-lab\start-gemma-lab.cmd first.",
        "At $clientScript"
    )) {
        if ($text.Contains($needle)) {
            throw "lab unreachable case included forbidden output: $needle"
        }
    }
}

function Invoke-ClientCase {
    param(
        [ValidateSet("success", "heartbeat", "comment_only", "cr_only_frames", "multiline_data", "empty_event_field", "no_colon_fields", "field_value_spacing", "business_cycle_payload", "error", "error_without_done", "http_error", "truncated", "final_without_done", "incomplete_frame", "timeout", "no_headers_timeout")]
        [string]$Name,
        [int]$TimeoutSeconds,
        [bool]$ExpectSuccess,
        [string[]]$MustContain,
        [string[]]$MustNotContain = @(),
        [string[]]$ExtraArgs = @()
    )

    Write-Host ""
    Write-Host "safety_case=$Name"
    $port = Get-FreeTcpPort
    $job = Start-FakeLab -Port $port -Mode $Name
    try {
        Wait-FakeLab -Port $port
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $argText = ($ExtraArgs | ForEach-Object { " $_" }) -join ""
            $output = & cmd.exe /c "`"$clientCmd`" -Lab `"http://127.0.0.1:$port`" -Prompt `"fake`" -TimeoutSeconds $TimeoutSeconds -ShowMeta$argText" 2>&1
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }

        $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            Write-Host $text.TrimEnd()
        }

        if ($ExpectSuccess -and $exitCode -ne 0) {
            throw "case '$Name' expected success but exited $exitCode"
        }
        if (-not $ExpectSuccess -and $exitCode -eq 0) {
            throw "case '$Name' expected failure but succeeded"
        }
        foreach ($needle in $MustContain) {
            if (-not $text.Contains($needle)) {
                throw "case '$Name' missing expected output: $needle"
            }
        }
        foreach ($needle in $MustNotContain) {
            if ($text.Contains($needle)) {
                throw "case '$Name' included forbidden output: $needle"
            }
        }
    } finally {
        Stop-Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job $job -Force -ErrorAction SilentlyContinue | Out-Null
    }
}

function Invoke-HelpCase {
    Write-Host ""
    Write-Host "safety_case=help"
    $output = & cmd.exe /c "`"$clientCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Stream one prompt through a running rustgpt-lab Web Lab without starting Gemma.",
        ".\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt",
        "-Output <mode>          raw or enhanced answer view, default raw",
        "-Profile <name>         request profile sent to rust-norion, default coding",
        "-TimeoutSeconds <n>     total PowerShell SSE client wait window, default 900; not a socket read poll",
        "generation budget sent as max_tokens, default 262144; not context message count",
        "-FeedbackAmount <n>     business-cycle feedback amount, default 0.5",
        "-NoSelfImprove          send self_improve=false for business-cycle",
        "-RustCheckCode <code>   optional Rust code sent to business-cycle checks",
        "exits nonzero on SSE error, timeout, EOF before done, or incomplete SSE frames"
    )) {
        if (-not $text.Contains($needle)) {
            throw "help case missing expected output: $needle"
        }
    }
}

function Invoke-SafetyHelpCase {
    Write-Host ""
    Write-Host "safety_case=safety_help"
    $output = & cmd.exe /c "`"$safetyCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "safety help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Validate rustgpt-lab wrapper safety and chat SSE client behavior offline.",
        ".\tools\rustgpt-lab\test-gemma-lab-safety.cmd",
        ".\tools\rustgpt-lab\test-chat-gemma-lab-client.cmd",
        "The older test-chat-gemma-lab-client.cmd name remains as a compatibility alias.",
        "The Web UI parser and interaction checks require Node.js on PATH.",
        "This helper does not SSH, start or stop Gemma, or send real inference prompts.",
        "Gemma and built-in wrapper help locks the 7878 backend, 8787 Web Lab, and 8686 runtime port map",
        "Gemma and built-in start wrappers support read-only -CheckOnly",
        "Gemma and built-in status/stop wrappers stay read-only or dry-run on random ports",
        "HTTP stream setup failures exit nonzero with the status and body",
        "EOF before done, including after final, or inside a frame exits nonzero as truncated",
        "Web UI SSE parser handles core frame edges and retains incomplete frames",
        "Web UI Enter/Shift+Enter, send/input state, context-window clamp, cancel/error/truncated draft restore, heartbeat progress, auto-scroll, clear-context including mid-stream clears, busy/readiness/safe-device/experience preflight gate blocks, HTTP failure recovery, rejected low-window history preservation, and rejected-stream context safety are covered offline",
        "REPL -SkipStart stays attach-only: a missing backend exits before any Gemma/start/REPL path",
        "PowerShell chat client sends business-cycle endpoint, output, profile, Rust check, self-improve, and feedback payload fields",
        "PowerShell chat client stops before /api/chat-stream when Web Lab is unreachable or backend busy/readiness/safe-device/experience/Gemma runtime preflight fails"
    )) {
        if (-not $text.Contains($needle)) {
            throw "safety help case missing expected output: $needle"
        }
    }
}

function Invoke-WebSseParserCase {
    Write-Host ""
    Write-Host "safety_case=web_sse_parser"
    $node = Get-Command node -ErrorAction SilentlyContinue
    if (-not $node) {
        throw "Node.js is required on PATH for web_sse_parser safety case"
    }

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $syntaxOutput = & node --check $webAppScript 2>&1
        $syntaxExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $syntaxText = ($syntaxOutput | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($syntaxText)) {
        Write-Host $syntaxText.TrimEnd()
    }
    if ($syntaxExitCode -ne 0) {
        throw "web_sse_parser app.js syntax check failed with exit code $syntaxExitCode"
    }

    $script = @'
const fs = require("fs");
const source = fs.readFileSync(process.argv[2], "utf8");
const parseMatch = source.match(/    function parseSse\(buffer, onEvent\) \{[\s\S]*?^    \}/m);
const dataMatch = source.match(/    function parseSseData\(value\) \{[\s\S]*?^    \}/m);
const boundaryMatch = source.match(/    function nextSseBoundary\(buffer\) \{[\s\S]*?^    \}/m);
if (!parseMatch || !dataMatch || !boundaryMatch) {
  throw new Error("failed to extract Web SSE parser functions");
}
eval(`${parseMatch[0]}\n${dataMatch[0]}\n${boundaryMatch[0]}`);

function assertEvents(name, input, expected, expectedRest = "") {
  const events = [];
  const rest = parseSse(input, (event, data) => {
    events.push([event, data]);
  });
  if (rest !== expectedRest) {
    throw new Error(`${name}: expected parser rest ${JSON.stringify(expectedRest)}, got ${JSON.stringify(rest)}`);
  }
  if (JSON.stringify(events) !== JSON.stringify(expected)) {
    throw new Error(`${name}: unexpected events ${JSON.stringify(events)}`);
  }
}

assertEvents("cr-only frames", "event: delta\rdata: cr only\r\revent: done\rdata: [DONE]\r\r", [["delta", "cr only"], ["done", "[DONE]"]]);
assertEvents("comment-only frames", ": keep-alive\n\nevent: done\ndata: [DONE]\n\n", [["done", "[DONE]"]]);
assertEvents("multiline data", "event: delta\ndata: line one\ndata: line two\n\nevent: done\ndata: [DONE]\n\n", [["delta", "line one\nline two"], ["done", "[DONE]"]]);
assertEvents("empty event fallback", "event:\ndata: empty event became message\n\nevent: done\ndata: [DONE]\n\n", [["message", "empty event became message"], ["done", "[DONE]"]]);
assertEvents("no-colon fields", "event\ndata: no-colon event became message\n\nevent: status\ndata\n\nevent: done\ndata: [DONE]\n\n", [["message", "no-colon event became message"], ["status", ""], ["done", "[DONE]"]]);
assertEvents("field value spacing", "event: delta\ndata:   indented\n\nevent: done\ndata: [DONE]\n\n", [["delta", "  indented"], ["done", "[DONE]"]]);

const rest = parseSse("event: delta\ndata: partial", () => {});
if (rest !== "event: delta\ndata: partial") {
  throw new Error("incomplete SSE frame was not retained for truncation handling");
}

console.log("web SSE parser checks passed");
'@

    $tempScript = Join-Path ([System.IO.Path]::GetTempPath()) ("rustgpt-lab-web-sse-parser-{0}.js" -f ([guid]::NewGuid().ToString("N")))
    try {
        Set-Content -LiteralPath $tempScript -Value $script -Encoding UTF8
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & node $tempScript $webAppScript 2>&1
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            Write-Host $text.TrimEnd()
        }
    } finally {
        Remove-Item -LiteralPath $tempScript -Force -ErrorAction SilentlyContinue
    }
    if ($exitCode -ne 0) {
        throw "web_sse_parser case failed with exit code $exitCode"
    }
    if (-not $text.Contains("web SSE parser checks passed")) {
        throw "web_sse_parser case missing success marker"
    }
}

function Invoke-WebUiInteractionCase {
    Write-Host ""
    Write-Host "safety_case=web_ui_interactions"
    $node = Get-Command node -ErrorAction SilentlyContinue
    if (-not $node) {
        throw "Node.js is required on PATH for web_ui_interactions safety case"
    }

    $script = @'
const fs = require("fs");
const vm = require("vm");
const appPath = process.argv[2];
const source = fs.readFileSync(appPath, "utf8");
const encoder = new TextEncoder();

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

class ClassList {
  constructor(owner) {
    this.owner = owner;
    this.values = new Set();
  }

  setFromString(value) {
    this.values = new Set(String(value || "").split(/\s+/).filter(Boolean));
  }

  add(value) {
    this.values.add(value);
  }

  toggle(value, enabled) {
    if (enabled) this.values.add(value);
    else this.values.delete(value);
  }

  contains(value) {
    return this.values.has(value);
  }

  toString() {
    return Array.from(this.values).join(" ");
  }
}

class Element {
  constructor(id = "", tag = "div") {
    this.id = id;
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.parentNode = null;
    this.textContent = "";
    this.value = "";
    this.checked = false;
    this.disabled = false;
    this.hidden = false;
    this.style = {};
    this.listeners = new Map();
    this.scrollTop = 0;
    this.scrollHeight = 1000;
    this.clientHeight = 200;
    this.scrollCalls = 0;
    this.focusCalls = 0;
    this.submitRequests = 0;
    this.classList = new ClassList(this);
    Object.defineProperty(this, "className", {
      get: () => this.classList.toString(),
      set: (value) => this.classList.setFromString(value)
    });
    this.className = "";
  }

  appendChild(child) {
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  insertBefore(child, before) {
    child.parentNode = this;
    const index = this.children.indexOf(before);
    if (index < 0) this.children.push(child);
    else this.children.splice(index, 0, child);
    return child;
  }

  addEventListener(type, listener) {
    if (!this.listeners.has(type)) this.listeners.set(type, []);
    this.listeners.get(type).push(listener);
  }

  async dispatchEvent(event) {
    event.target = event.target || this;
    event.currentTarget = this;
    const results = [];
    for (const listener of this.listeners.get(event.type) || []) {
      results.push(listener(event));
    }
    await Promise.all(results);
    return !event.defaultPrevented;
  }

  requestSubmit() {
    this.submitRequests += 1;
  }

  scrollTo(options) {
    this.scrollCalls += 1;
    this.scrollTop = options && typeof options.top === "number" ? options.top : this.scrollHeight;
  }

  focus() {
    this.focusCalls += 1;
  }
}

const ids = [
  "chatForm",
  "prompt",
  "messages",
  "send",
  "cancel",
  "statusLine",
  "backendLine",
  "modelPoolLine",
  "modelPoolAdviceLine",
  "contextLine",
  "endpoint",
  "maxTokens",
  "contextLimit",
  "clearContext",
  "followOutput",
  "businessControls",
  "rustCheckCode",
  "profile",
  "outputMode",
  "feedbackAmount",
  "selfImprove"
];

function responseJson(value) {
  return {
    ok: true,
    json: async () => value
  };
}

function healthyBackendHealth(overrides = {}) {
  const baseHygiene = {
    clean: true,
    checked: true,
    quarantine_candidates: 0,
    repairable_legacy_metadata_lessons: 0,
    repairable_index_records: 0,
    index: { retrieval_ready: true, risk_level: "ok", quality_score: 1, noisy_records: 0, duplicate_outputs: 0 }
  };
  const hygieneOverride = overrides.experience_hygiene || {};
  return {
    ok: true,
    runtime_mode: "built-in",
    readiness_ok: true,
    safe_device_ok: true,
    engine_busy: false,
    active_engine_requests: 0,
    requests_seen: 1,
    ...overrides,
    experience_hygiene: {
      ...baseHygiene,
      ...hygieneOverride,
      index: {
        ...baseHygiene.index,
        ...(hygieneOverride.index || {})
      }
    }
  };
}

function streamResponse(text, options = {}, signal = null) {
  const chunks = text.split(/(?<=\n\n)/).filter(Boolean).map((part) => encoder.encode(part));
  let index = 0;
  let cancelled = false;
  let pendingRead = null;

  function resolvePendingDone() {
    if (pendingRead) {
      const resolve = pendingRead;
      pendingRead = null;
      resolve({ done: true });
    }
  }

  if (signal) {
    signal.addEventListener("abort", () => {
      cancelled = true;
      resolvePendingDone();
    });
  }

  return {
    ok: true,
    body: {
      getReader() {
        const reader = {
          async read() {
            if (cancelled) return { done: true };
            if (
              Number.isInteger(options.pauseAfterChunks)
              && index >= options.pauseAfterChunks
              && !options.streamResumed
            ) {
              return new Promise((resolve) => {
                options.resumeStream = () => {
                  options.streamResumed = true;
                  resolve(reader.read());
                };
              });
            }
            if (index >= chunks.length && options.holdOpenAfterChunks) {
              return new Promise((resolve) => {
                pendingRead = resolve;
              });
            }
            if (index >= chunks.length) return { done: true };
            return { value: chunks[index++], done: false };
          },
          async cancel() {
            cancelled = true;
            resolvePendingDone();
          }
        };
        return reader;
      }
    }
  };
}

function resetStreamPause(options) {
  delete options.resumeStream;
  options.streamResumed = false;
}

function createHarness(streamText, options = {}) {
  const elements = Object.fromEntries(ids.map((id) => [id, new Element(id)]));
  elements.endpoint.value = "chat";
  elements.maxTokens.value = "262144";
  elements.contextLimit.value = "64";
  elements.followOutput.checked = true;
  elements.profile.value = "coding";
  elements.outputMode.value = "raw";
  elements.feedbackAmount.value = "0.5";
  elements.selfImprove.checked = true;
  const main = new Element("main", "main");
  const requests = [];
  const backendHealthResponses = Array.isArray(options.backendHealthResponses)
    ? [...options.backendHealthResponses]
    : null;

  const document = {
    getElementById(id) {
      if (!elements[id]) elements[id] = new Element(id);
      return elements[id];
    },
    createElement(tag) {
      return new Element("", tag);
    },
    querySelector(selector) {
      if (selector === "main") return main;
      return null;
    }
  };

  async function fetchStub(url, init = {}) {
    if (url === "/api/backend-health") {
      if (backendHealthResponses) {
        return responseJson(backendHealthResponses.length > 1
          ? backendHealthResponses.shift()
          : backendHealthResponses[0]);
      }
      return responseJson(healthyBackendHealth());
    }
    if (url === "/api/model-pool-status") {
      return responseJson({ ok: true, worker_count: 0, healthy_worker_count: 0, workers: [], launch_allowed: true });
    }
    if (url === "/api/model-pool-advice") {
      return responseJson({ ok: true, advice: "\u6a21\u578b\u6c60\u5efa\u8bae\uff1aoffline", kind: "" });
    }
    if (url === "/api/chat-stream") {
      assert(elements.send.disabled === true, "send button was not disabled while stream was in flight");
      assert(elements.send.textContent === "\u53d1\u9001\u4e2d", `expected in-flight send label, got ${JSON.stringify(elements.send.textContent)}`);
      assert(elements.prompt.disabled === true, "prompt textarea should be disabled while stream is in flight");
      assert(elements.rustCheckCode.disabled === true, "Rust check textarea should be disabled while stream is in flight");
      const body = typeof init.body === "string" ? JSON.parse(init.body) : null;
      requests.push({ url, body });
      if (body) {
        assert(elements.statusLine.textContent.includes(`\u53d1\u9001 ${body.messages.length} \u6761\u6d88\u606f`), `send status should describe total request messages, got ${elements.statusLine.textContent}`);
        assert(!elements.statusLine.textContent.includes("\u4e0a\u4e0b\u6587\u6d88\u606f"), `send status should not call request messages context messages: ${elements.statusLine.textContent}`);
      }
      if (options.httpStatus) return { ok: false, status: options.httpStatus };
      const responseText = Array.isArray(options.streamTexts)
        ? (options.streamTexts.length > 1 ? options.streamTexts.shift() : options.streamTexts[0])
        : streamText;
      return streamResponse(responseText, options, init.signal);
    }
    throw new Error(`unexpected fetch: ${url}`);
  }

  const window = {};
  const context = {
    window,
    document,
    fetch: fetchStub,
    requestAnimationFrame: (callback) => callback(),
    setTimeout: () => 1,
    clearTimeout: () => {},
    setInterval: () => 1,
    clearInterval: () => {},
    TextDecoder,
    AbortController,
    console
  };
  window.document = document;
  Object.assign(window, context);
  vm.runInNewContext(source, context, { filename: appPath });
  return { elements, main, requests };
}

async function flushAppInit() {
  for (let i = 0; i < 8; i += 1) {
    await Promise.resolve();
  }
}

async function submitPrompt(elements, prompt = "hello") {
  elements.prompt.value = prompt;
  await elements.chatForm.dispatchEvent({
    type: "submit",
    preventDefault() {
      this.defaultPrevented = true;
    }
  });
}

function findMessage(elements, className) {
  return elements.messages.children.find((child) => child.classList.contains(className));
}

async function assertReady(elements) {
  await flushAppInit();
  assert(elements.send.disabled === false, "send button should be enabled after healthy offline preflight");
  assert(elements.send.textContent === "\u53d1\u9001", `unexpected ready send label: ${elements.send.textContent}`);
}

async function assertComposerKeys(elements) {
  elements.prompt.value = "hello";
  let enterPrevented = false;
  await elements.prompt.dispatchEvent({
    type: "keydown",
    key: "Enter",
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    repeat: false,
    keyCode: 13,
    preventDefault() {
      enterPrevented = true;
      this.defaultPrevented = true;
    }
  });
  assert(enterPrevented, "plain Enter did not prevent default");
  assert(elements.chatForm.submitRequests === 1, "plain Enter did not request form submit");

  let shiftPrevented = false;
  await elements.prompt.dispatchEvent({
    type: "keydown",
    key: "Enter",
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    repeat: false,
    keyCode: 13,
    preventDefault() {
      shiftPrevented = true;
      this.defaultPrevented = true;
    }
  });
  assert(!shiftPrevented, "Shift+Enter should be left to the textarea for newline input");
  assert(elements.chatForm.submitRequests === 1, "Shift+Enter should not request submit");

  async function assertEnterDoesNotSubmit(name, overrides) {
    let prevented = false;
    await elements.prompt.dispatchEvent({
      type: "keydown",
      key: "Enter",
      shiftKey: false,
      ctrlKey: false,
      altKey: false,
      metaKey: false,
      isComposing: false,
      repeat: false,
      keyCode: 13,
      ...overrides,
      preventDefault() {
        prevented = true;
        this.defaultPrevented = true;
      }
    });
    assert(!prevented, `${name} should not prevent default`);
    assert(elements.chatForm.submitRequests === 1, `${name} should not request submit`);
  }

  await assertEnterDoesNotSubmit("IME composing Enter", { isComposing: true });
  await assertEnterDoesNotSubmit("IME keyCode 229 Enter", { keyCode: 229 });
  await assertEnterDoesNotSubmit("repeat Enter", { repeat: true });
  await assertEnterDoesNotSubmit("Ctrl+Enter", { ctrlKey: true });
  await assertEnterDoesNotSubmit("Alt+Enter", { altKey: true });
  await assertEnterDoesNotSubmit("Meta+Enter", { metaKey: true });
}
async function assertRustCheckComposerKeys(elements) {
  elements.rustCheckCode.value = "pub fn ok() -> bool { true }";
  const baselineSubmits = elements.chatForm.submitRequests;
  let enterPrevented = false;
  await elements.rustCheckCode.dispatchEvent({
    type: "keydown",
    key: "Enter",
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    repeat: false,
    keyCode: 13,
    preventDefault() {
      enterPrevented = true;
      this.defaultPrevented = true;
    }
  });
  assert(enterPrevented, "Rust check plain Enter did not prevent default");
  assert(elements.chatForm.submitRequests === baselineSubmits + 1, "Rust check plain Enter did not request form submit");

  async function assertRustEnterDoesNotSubmit(name, overrides) {
    let prevented = false;
    await elements.rustCheckCode.dispatchEvent({
      type: "keydown",
      key: "Enter",
      shiftKey: false,
      ctrlKey: false,
      altKey: false,
      metaKey: false,
      isComposing: false,
      repeat: false,
      keyCode: 13,
      ...overrides,
      preventDefault() {
        prevented = true;
        this.defaultPrevented = true;
      }
    });
    assert(!prevented, `${name} should not prevent default in Rust check textarea`);
    assert(elements.chatForm.submitRequests === baselineSubmits + 1, `${name} should not request submit from Rust check textarea`);
  }

  await assertRustEnterDoesNotSubmit("Rust check Shift+Enter", { shiftKey: true });
  await assertRustEnterDoesNotSubmit("Rust check IME composing Enter", { isComposing: true });
  await assertRustEnterDoesNotSubmit("Rust check repeat Enter", { repeat: true });
  await assertRustEnterDoesNotSubmit("Rust check Ctrl+Enter", { ctrlKey: true });
}

async function assertSuccessScenario() {
  const { elements, main, requests } = createHarness([
    "event: status\ndata: checking backend prompt gate before forwarding request\n\n",
    "event: heartbeat\ndata: waiting on fake backend\n\n",
    "event: delta\ndata: offline answer\n\n",
    "event: final\ndata: {\"answer\":\"offline answer\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\n",
    "event: done\ndata: [DONE]\n\n"
  ].join(""));

  await assertReady(elements);
  await assertComposerKeys(elements);
  await submitPrompt(elements);

  assert(elements.send.disabled === false, "send button should re-enable after stream completion");
  assert(elements.prompt.disabled === false, "prompt textarea should recover after stream completion");
  assert(elements.rustCheckCode.disabled === false, "Rust check textarea should recover after stream completion");
  assert(elements.send.textContent === "\u53d1\u9001", `send label should recover after stream completion: ${elements.send.textContent}`);
  assert(elements.statusLine.textContent === "\u5b8c\u6210", `expected final status complete, got ${JSON.stringify(elements.statusLine.textContent)}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a2/64"), `context line did not include committed user+assistant messages: ${elements.contextLine.textContent}`);
  assert(main.scrollCalls > 0, "auto-scroll did not scroll during streamed output");
  assert(main.scrollTop === main.scrollHeight, "auto-scroll did not land at the bottom");

  const assistant = findMessage(elements, "assistant");
  assert(assistant && assistant.textContent === "offline answer", "assistant stream text was not rendered");
  assert(!assistant.classList.contains("interrupted"), "successful done stream should not leave assistant interrupted");
  const progress = findMessage(elements, "progress");
  assert(progress && progress.textContent === "heartbeat: waiting on fake backend", "heartbeat/status progress row was not visible or upserted");
  assert(requests.length === 1, `expected one chat-stream request, got ${requests.length}`);
  assert(requests[0].body.messages.length === 1, `first request should send only the new user prompt, got ${requests[0].body.messages.length}`);
  assert(requests[0].body.messages[0].role === "user", "first request should end with the user prompt");
  assert(requests[0].body.max_tokens === 262144, `default generation budget should stay max_tokens=262144, got ${requests[0].body.max_tokens}`);
}

async function assertBusinessCycleRustCheckPayload() {
  const { elements, requests } = createHarness([
    "event: stage\ndata: rust check queued\n\n",
    "event: delta\ndata: business answer\n\n",
    "event: final\ndata: {\"answer\":\"business answer\",\"elapsed_ms\":9,\"runtime_token_count\":4}\n\n",
    "event: done\ndata: [DONE]\n\n"
  ].join(""));

  await assertReady(elements);
  elements.endpoint.value = "business-cycle";
  await elements.endpoint.dispatchEvent({ type: "change" });
  assert(elements.businessControls.classList.contains("active"), "business controls should become active for business-cycle endpoint");
  assert(elements.rustCheckCode.style.display === "block", "Rust check textarea should show for business-cycle endpoint");
  await assertRustCheckComposerKeys(elements);
  elements.rustCheckCode.value = "pub fn ok() -> bool { true }";
  await submitPrompt(elements, "run business cycle");

  assert(requests.length === 1, `expected one business-cycle request, got ${requests.length}`);
  assert(requests[0].body.endpoint === "business-cycle", `expected business-cycle endpoint, got ${requests[0].body.endpoint}`);
  assert(requests[0].body.rust_check_code === "pub fn ok() -> bool { true }", `Rust check code was not sent in payload: ${requests[0].body.rust_check_code}`);
  assert(requests[0].body.self_improve === true, "business-cycle payload should preserve self_improve checkbox");
  assert(requests[0].body.feedback_amount === "0.5", `business-cycle payload should preserve feedback amount, got ${requests[0].body.feedback_amount}`);
}

async function assertContextLimitMeansRequestMessageSlots() {
  const { elements, requests } = createHarness([
    "event: delta\ndata: first answer\n\n",
    "event: final\ndata: {\"answer\":\"first answer\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\n",
    "event: done\ndata: [DONE]\n\n"
  ].join(""));

  await assertReady(elements);
  elements.contextLimit.value = "2";
  elements.maxTokens.value = "8192";
  await submitPrompt(elements, "first question");
  await submitPrompt(elements, "second question");

  assert(requests.length === 2, `expected two chat-stream requests, got ${requests.length}`);
  assert(requests[0].body.messages.length === 1, `first limited request should send one message, got ${requests[0].body.messages.length}`);
  assert(requests[1].body.messages.length === 2, `context limit 2 should send one prior message plus current prompt, got ${requests[1].body.messages.length}`);
  assert(requests[1].body.messages[0].role === "assistant", `prior message should be retained from completed conversation, got ${requests[1].body.messages[0].role}`);
  assert(requests[1].body.messages[1].role === "user", "last request message should be the current user prompt");
  assert(requests[1].body.prompt === "second question", `prompt field should remain the current user prompt, got ${requests[1].body.prompt}`);
  assert(requests[1].body.max_tokens === 8192, `max_tokens should remain the generation budget, got ${requests[1].body.max_tokens}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a2/2"), `context line should display committed short chat messages, got ${elements.contextLine.textContent}`);
}

async function assertContextLimitClampMatchesControlRange() {
  const { elements } = createHarness("");

  await assertReady(elements);
  elements.contextLimit.value = "1";
  await elements.contextLimit.dispatchEvent({ type: "change" });

  assert(elements.contextLimit.value === "2", `context message control should clamp to UI min 2, got ${elements.contextLimit.value}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/2"), `context line should show message slots after low clamp: ${elements.contextLine.textContent}`);
  assert(elements.contextLine.textContent.includes("max_tokens=262144"), `context line should keep generation budget separate: ${elements.contextLine.textContent}`);
}

async function assertRejectedLowWindowSendDoesNotTrimCompletedHistory() {
  const { elements, requests } = createHarness("", {
    streamTexts: [
      "event: final\ndata: {\"answer\":\"seed answer one\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\nevent: done\ndata: [DONE]\n\n",
      "event: final\ndata: {\"answer\":\"seed answer two\",\"elapsed_ms\":8,\"runtime_token_count\":4}\n\nevent: done\ndata: [DONE]\n\n",
      "event: error\ndata: rejected while probing low window\n\nevent: done\ndata: [DONE]\n\n",
      "event: final\ndata: {\"answer\":\"history still intact\",\"elapsed_ms\":9,\"runtime_token_count\":5}\n\nevent: done\ndata: [DONE]\n\n"
    ]
  });

  await assertReady(elements);
  await submitPrompt(elements, "seed one");
  await submitPrompt(elements, "seed two");
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a4/64"), `two completed turns should seed four messages: ${elements.contextLine.textContent}`);

  elements.contextLimit.value = "2";
  await submitPrompt(elements, "rejected low-window probe");
  assert(elements.prompt.value === "rejected low-window probe", "rejected low-window send should restore draft");
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a2/2"), `low-window rejection should display the current window without committing the turn: ${elements.contextLine.textContent}`);

  elements.contextLimit.value = "64";
  await elements.contextLimit.dispatchEvent({ type: "change" });
  await submitPrompt(elements, "history check after rejection");

  assert(requests.length === 4, `expected four requests, got ${requests.length}`);
  assert(requests[2].body.messages.length === 2, `rejected low-window request should send one prior message plus prompt, got ${requests[2].body.messages.length}`);
  assert(requests[3].body.messages.length === 5, `history after rejected low-window request should still include four completed messages plus prompt, got ${requests[3].body.messages.length}`);
  assert(requests[3].body.messages[0].content === "seed one", `first completed user turn was trimmed by rejected stream: ${requests[3].body.messages[0].content}`);
  assert(requests[3].body.messages[4].content === "history check after rejection", "last request message should be the new history-check prompt");
}

async function assertClearContextResetsNextRequestHistory() {
  const { elements, requests } = createHarness([
    "event: delta\ndata: answer after clear check\n\n",
    "event: final\ndata: {\"answer\":\"answer after clear check\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\n",
    "event: done\ndata: [DONE]\n\n"
  ].join(""));

  await assertReady(elements);
  await submitPrompt(elements, "first remembered turn");
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a2/64"), `completed turn should be in context before clearing: ${elements.contextLine.textContent}`);

  await elements.clearContext.dispatchEvent({ type: "click" });
  assert(elements.statusLine.textContent === "\u4e0a\u4e0b\u6587\u5df2\u6e05\u7a7a", `clear-context status should be visible, got ${elements.statusLine.textContent}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `clear-context should reset context line: ${elements.contextLine.textContent}`);
  assert(elements.messages.children.some((child) => child.textContent === "context cleared"), "clear-context meta row should be visible");

  await submitPrompt(elements, "fresh turn after clear");
  assert(requests.length === 2, `expected two requests around clear-context, got ${requests.length}`);
  assert(requests[1].body.messages.length === 1, `clear-context should make next request send only the current user prompt, got ${requests[1].body.messages.length}`);
  assert(requests[1].body.messages[0].role === "user", "clear-context next request should contain only the new user role");
  assert(requests[1].body.messages[0].content === "fresh turn after clear", `clear-context next request should contain the new prompt, got ${requests[1].body.messages[0].content}`);
}

async function assertClearContextDuringInFlightStaysCleared() {
  const streamOptions = {};
  const { elements, requests } = createHarness([
    "event: delta\ndata: answer around in-flight clear\n\n",
    "event: final\ndata: {\"answer\":\"answer around in-flight clear\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\n",
    "event: done\ndata: [DONE]\n\n"
  ].join(""), streamOptions);

  await assertReady(elements);
  await submitPrompt(elements, "seed history before in-flight clear");
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a2/64"), `seed turn should commit before in-flight clear test: ${elements.contextLine.textContent}`);

  streamOptions.pauseAfterChunks = 1;
  resetStreamPause(streamOptions);
  const submitTask = submitPrompt(elements, "clear while stream is running");
  await flushAppInit();
  await flushAppInit();

  assert(typeof streamOptions.resumeStream === "function", "fake stream did not pause before final/done");
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a3/64"), `in-flight request preview should include prior context before clearing: ${elements.contextLine.textContent}`);
  await elements.clearContext.dispatchEvent({ type: "click" });
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `mid-stream clear-context should reset context immediately: ${elements.contextLine.textContent}`);

  streamOptions.resumeStream();
  await submitTask;

  assert(requests.length === 2, `expected two requests around in-flight clear, got ${requests.length}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `completed stream after mid-stream clear should not restore old context: ${elements.contextLine.textContent}`);
  assert(elements.messages.children.some((child) => child.textContent.includes("context cleared during stream")), "missing evidence that completed turn was not committed after mid-stream clear");
  assert(elements.send.disabled === false, "send button should recover after mid-stream clear scenario");
  assert(elements.prompt.disabled === false, "prompt textarea should recover after mid-stream clear scenario");
}

async function assertPreflightBlockedScenario(name, blockedHealth, expectedStatusFragment, expectedMetaFragment, expectedSendLabel) {
  const { elements, requests } = createHarness("", {
    backendHealthResponses: [healthyBackendHealth(), blockedHealth]
  });
  await assertReady(elements);
  await submitPrompt(elements, name);

  assert(requests.length === 0, `${name}: blocked preflight should not call /api/chat-stream`);
  assert(elements.prompt.value === name, `${name}: blocked preflight should keep the draft prompt`);
  assert(elements.send.disabled === true, `${name}: send button should remain gated after blocked preflight`);
  assert(elements.send.textContent === expectedSendLabel, `${name}: send label should explain gate, got ${elements.send.textContent}`);
  assert(elements.prompt.disabled === false, `${name}: prompt textarea should stay editable after blocked preflight`);
  assert(elements.rustCheckCode.disabled === false, `${name}: Rust check textarea should stay editable after blocked preflight`);
  assert(elements.cancel.hidden === true, `${name}: cancel button should stay hidden when stream never starts`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `${name}: blocked preflight should not commit context: ${elements.contextLine.textContent}`);
  assert(elements.statusLine.textContent.includes(expectedStatusFragment), `${name}: status did not explain blocked preflight: ${elements.statusLine.textContent}`);
  assert(elements.messages.children.some((child) => child.textContent.includes(expectedMetaFragment)), `${name}: missing blocked preflight meta row`);
  assert(!findMessage(elements, "user"), `${name}: blocked preflight should not append a user message`);
  assert(!findMessage(elements, "assistant"), `${name}: blocked preflight should not append an assistant message`);
}

async function assertRejectedScenario(name, streamText, expectedStatusFragment, expectedMetaFragment, expectedPartialFragment, options = {}) {
  const { elements } = createHarness(streamText, options);
  await assertReady(elements);
  await submitPrompt(elements, name);

  assert(elements.send.disabled === false, `${name}: send button should recover after rejected stream`);
  assert(elements.prompt.disabled === false, `${name}: prompt textarea should recover after rejected stream`);
  assert(elements.prompt.value === name, `${name}: rejected stream should restore the draft prompt`);
  assert(elements.rustCheckCode.disabled === false, `${name}: Rust check textarea should recover after rejected stream`);
  assert(elements.prompt.focusCalls > 0, `${name}: composer focus should recover after rejected stream`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `${name}: rejected stream should not commit user or assistant context: ${elements.contextLine.textContent}`);
  assert(elements.statusLine.textContent.includes(expectedStatusFragment), `${name}: status did not expose rejection: ${elements.statusLine.textContent}`);
  const assistant = findMessage(elements, "assistant");
  assert(assistant, `${name}: assistant bubble should remain visible after rejection`);
  if (expectedPartialFragment !== null) {
    assert(assistant.textContent.includes(expectedPartialFragment), `${name}: partial output should remain visible in the assistant bubble`);
  }
  assert(assistant && assistant.classList.contains("interrupted"), `${name}: assistant bubble should be marked interrupted`);
  assert(elements.messages.children.some((child) => child.textContent.includes(expectedMetaFragment)), `${name}: missing interruption evidence meta row`);
}

async function assertCancelScenario() {
  const { elements } = createHarness(
    "event: delta\ndata: partial before cancel\n\n",
    { holdOpenAfterChunks: true }
  );
  await assertReady(elements);
  const submitTask = submitPrompt(elements, "cancel me");
  await flushAppInit();
  await flushAppInit();

  assert(elements.send.disabled === true, "send button should be disabled while stream is in flight");
  assert(elements.send.textContent === "\u53d1\u9001\u4e2d", `send label should show in-flight state, got ${elements.send.textContent}`);
  assert(elements.prompt.disabled === true, "prompt textarea should be disabled while stream is in flight");
  assert(elements.rustCheckCode.disabled === true, "Rust check textarea should be disabled while stream is in flight");
  assert(elements.cancel.hidden === false, "cancel button should be visible while stream is in flight");
  assert(elements.cancel.disabled === false, "cancel button should be enabled while stream is cancellable");
  const assistant = findMessage(elements, "assistant");
  assert(assistant && assistant.textContent.includes("partial before cancel"), "partial output should render before cancel");

  await elements.cancel.dispatchEvent({ type: "click" });
  await submitTask;

  assert(elements.send.disabled === false, "send button should recover after cancel");
  assert(elements.prompt.disabled === false, "prompt textarea should recover after cancel");
  assert(elements.prompt.value === "cancel me", "cancelled stream should restore the draft prompt");
  assert(elements.rustCheckCode.disabled === false, "Rust check textarea should recover after cancel");
  assert(elements.cancel.hidden === true, "cancel button should hide after cancel completes");
  assert(elements.prompt.focusCalls > 0, "composer focus should recover after cancel");
  assert(elements.statusLine.textContent === "\u5df2\u53d6\u6d88\u5f53\u524d\u8bf7\u6c42", `cancel status should be visible, got ${elements.statusLine.textContent}`);
  assert(elements.contextLine.textContent.includes("\u4e0a\u4e0b\u6587\uff1a0/64"), `cancelled stream should not commit context: ${elements.contextLine.textContent}`);
  assert(assistant.classList.contains("interrupted"), "cancelled assistant bubble should be marked interrupted");
  assert(elements.messages.children.some((child) => child.textContent.includes("cancel requested by user")), "cancel request meta row should remain visible");
}

(async () => {
  await assertSuccessScenario();
  await assertBusinessCycleRustCheckPayload();
  await assertContextLimitMeansRequestMessageSlots();
  await assertContextLimitClampMatchesControlRange();
  await assertRejectedLowWindowSendDoesNotTrimCompletedHistory();
  await assertClearContextResetsNextRequestHistory();
  await assertClearContextDuringInFlightStaysCleared();
  await assertPreflightBlockedScenario(
    "busy preflight prompt",
    healthyBackendHealth({ engine_busy: true, active_engine_requests: 1 }),
    "\u540e\u7aef\u6b63\u5fd9\u6216\u4e0d\u53ef\u7528",
    "blocked: backend is busy or unavailable",
    "\u540e\u7aef\u5fd9"
  );
  await assertPreflightBlockedScenario(
    "readiness blocked prompt",
    healthyBackendHealth({ readiness_ok: false }),
    "\u540e\u7aef\u9884\u68c0\u672a\u901a\u8fc7",
    "blocked: backend readiness",
    "\u9884\u68c0\u5931\u8d25"
  );
  await assertPreflightBlockedScenario(
    "safe-device blocked prompt",
    healthyBackendHealth({ safe_device_ok: false, safe_device_failures: ["gpu memory below threshold"] }),
    "\u540e\u7aef\u9884\u68c0\u672a\u901a\u8fc7",
    "blocked: backend readiness",
    "\u9884\u68c0\u5931\u8d25"
  );
  await assertPreflightBlockedScenario(
    "experience hygiene blocked prompt",
    healthyBackendHealth({
      experience_hygiene: {
        clean: false,
        quarantine_candidates: 1,
        index: { retrieval_ready: false, risk_level: "blocked" }
      }
    }),
    "\u540e\u7aef\u9884\u68c0\u672a\u901a\u8fc7",
    "blocked: backend readiness",
    "\u9884\u68c0\u5931\u8d25"
  );
  await assertCancelScenario();
  await assertRejectedScenario(
    "sse-error",
    "event: delta\ndata: partial before error\n\nevent: error\ndata: blocked by fake gate\n\nevent: done\ndata: [DONE]\n\n",
    "\u9519\u8bef\uff1ablocked by fake gate",
    "error: blocked by fake gate",
    "partial before error"
  );
  await assertRejectedScenario(
    "truncated-eof",
    "event: delta\ndata: partial without done\n\n",
    "\u6d41\u5f0f\u8fde\u63a5\u4e2d\u65ad",
    "context not updated",
    "partial without done"
  );
  await assertRejectedScenario(
    "final-without-done",
    "event: final\ndata: {\"answer\":\"final without done\",\"elapsed_ms\":7,\"runtime_token_count\":3}\n\n",
    "\u6d41\u5f0f\u8fde\u63a5\u4e2d\u65ad",
    "context not updated",
    "final without done"
  );
  await assertRejectedScenario(
    "http-failure",
    "",
    "\u6d41\u5f0f\u8fde\u63a5\u4e2d\u65ad\uff1aHTTP 503",
    "context not updated",
    null,
    { httpStatus: 503 }
  );

  console.log("web UI interaction checks passed");
})().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});
'@

    $tempScript = Join-Path ([System.IO.Path]::GetTempPath()) ("rustgpt-lab-web-ui-interactions-{0}.js" -f ([guid]::NewGuid().ToString("N")))
    try {
        Set-Content -LiteralPath $tempScript -Value $script -Encoding UTF8
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & node $tempScript $webAppScript 2>&1
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            Write-Host $text.TrimEnd()
        }
    } finally {
        Remove-Item -LiteralPath $tempScript -Force -ErrorAction SilentlyContinue
    }
    if ($exitCode -ne 0) {
        throw "web_ui_interactions case failed with exit code $exitCode"
    }
    if (-not $text.Contains("web UI interaction checks passed")) {
        throw "web_ui_interactions case missing success marker"
    }
}

function Invoke-ReplHelpCase {
    Write-Host ""
    Write-Host "safety_case=repl_help"
    $output = & cmd.exe /c "`"$replCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "repl help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Open the rustgpt-lab REPL against Gemma through rust-norion.",
        ".\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart",
        "Without -SkipStart, this script calls the Gemma start helper",
        "With -SkipStart, it only attaches",
        "REPL short-context message count, default 64; not a token limit",
        "rustgpt-lab -> rust-norion total streaming window, default 900"
    )) {
        if (-not $text.Contains($needle)) {
            throw "repl help case missing expected output: $needle"
        }
    }
}

function Invoke-ReplSkipStartBackendMissingCase {
    Write-Host ""
    Write-Host "safety_case=repl_skip_start_backend_missing"
    $backendPort = Get-FreeTcpPort
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & cmd.exe /c "`"$replCmd`" -SkipStart -BackendPort $backendPort" 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -eq 0) {
        throw "repl skip-start backend-missing case expected failure but succeeded"
    }
    foreach ($needle in @(
        "rust-norion backend is not listening on 127.0.0.1:$backendPort.",
        "-SkipStart is attach-only",
        "starts no model",
        "status-gemma-lab.cmd",
        "status-built-in-lab.cmd",
        "start-built-in-lab.cmd",
        "Only omit -SkipStart when you intentionally want the Gemma lab start helper",
        "Port 8686 is the optional Gemma runtime behind rust-norion, not a REPL prompt target."
    )) {
        if (-not $text.Contains($needle)) {
            throw "repl skip-start backend-missing case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Starting Gemma",
        "rust-norion pid:",
        "rustgpt-lab pid:",
        "mistralrs pid:",
        "Opening rustgpt-lab REPL",
        "At $replCmd",
        "At $RepoRoot"
    )) {
        if ($text.Contains($needle)) {
            throw "repl skip-start backend-missing case included forbidden output: $needle"
        }
    }
}

function Invoke-StartHelpCase {
    Write-Host ""
    Write-Host "safety_case=start_help"
    $output = & cmd.exe /c "`"$startCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "start help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Start the real Gemma 12B rust-norion + rustgpt-lab stack.",
        ".\tools\rustgpt-lab\start-gemma-lab.cmd -CheckOnly",
        "Without -CheckOnly, this script can build binaries and start Gemma",
        "-CheckOnly is read-only",
        "Port map:",
        "7878 = rust-norion backend; Web Lab forwards prompts there after gates.",
        "8787 = rustgpt-lab browser UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime behind rust-norion; do not send prompts there directly.",
        "-RuntimeTimeoutMs <ms>           rust-norion -> Gemma runtime request timeout, not the Web Lab read poll.",
        "-LabBackendTimeoutSeconds <sec>  rustgpt-lab -> rust-norion total streaming window."
    )) {
        if (-not $text.Contains($needle)) {
            throw "start help case missing expected output: $needle"
        }
    }
}

function Invoke-StatusHelpCase {
    Write-Host ""
    Write-Host "safety_case=status_help"
    $output = & cmd.exe /c "`"$statusCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "status help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Show the local Gemma/rust-norion/rustgpt-lab test-stack status.",
        "This is read-only. It does not start Gemma, stop processes, or write .ndkv files.",
        ".\tools\rustgpt-lab\status-gemma-lab.cmd",
        "-MistralPort <n>  optional Gemma/mistralrs runtime port, default 8686",
        "-BackendPort <n>  rust-norion model-service backend port, default 7878",
        "-LabPort <n>      rustgpt-lab Web UI/SSE proxy port, default 8787",
        "7878 = rust-norion backend; Web Lab forwards prompts there after gates.",
        "8787 = rustgpt-lab browser UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime behind rust-norion; do not send prompts there directly."
    )) {
        if (-not $text.Contains($needle)) {
            throw "status help case missing expected output: $needle"
        }
    }
}

function Invoke-StopHelpCase {
    Write-Host ""
    Write-Host "safety_case=stop_help"
    $output = & cmd.exe /c "`"$stopCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "stop help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Stop the local Gemma/rust-norion/rustgpt-lab test stack.",
        "Port map:",
        "7878 = rust-norion backend; Web Lab forwards prompts there after gates.",
        "8787 = rustgpt-lab browser UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime behind rust-norion; -KeepMistral leaves it running.",
        ".\tools\rustgpt-lab\stop-gemma-lab.cmd -DryRun",
        ".\tools\rustgpt-lab\stop-gemma-lab.cmd -KeepMistral",
        ".\tools\rustgpt-lab\stop-gemma-lab.cmd -ForceAll"
    )) {
        if (-not $text.Contains($needle)) {
            throw "stop help case missing expected output: $needle"
        }
    }
    if ($text.Contains("tools\smartsteam-forge\stop-forge.cmd")) {
        throw "stop help case still references smartsteam-forge stop wrapper"
    }
}

function Invoke-StatusReadOnlyCase {
    Write-Host ""
    Write-Host "safety_case=status_read_only"
    $mistralPort = Get-FreeTcpPort
    $backendPort = Get-FreeTcpPort
    $labPort = Get-FreeTcpPort

    $output = & cmd.exe /c "`"$statusCmd`" -MistralPort $mistralPort -BackendPort $backendPort -LabPort $labPort" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "status read-only case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Gemma lab status",
        "mistralrs",
        "rust-norion",
        "rustgpt-lab",
        "GPU summary:"
    )) {
        if (-not $text.Contains($needle)) {
            throw "status read-only case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Starting Gemma 12B runtime",
        "Stopping "
    )) {
        if ($text.Contains($needle)) {
            throw "status read-only case included forbidden side-effect output: $needle"
        }
    }
}

function Invoke-StopDryRunCase {
    Write-Host ""
    Write-Host "safety_case=stop_dry_run"
    $mistralPort = Get-FreeTcpPort
    $backendPort = Get-FreeTcpPort
    $labPort = Get-FreeTcpPort

    $output = & cmd.exe /c "`"$stopCmd`" -MistralPort $mistralPort -BackendPort $backendPort -LabPort $labPort -DryRun" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "stop dry-run case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Skipping backend port ${backendPort}: /health did not confirm rust-norion runtime_mode=gemma-http or built-in.",
        "Skipping Web Lab port ${labPort}: /health did not confirm rustgpt-lab backend=127.0.0.1:$backendPort.",
        "No confirmed local test-stack processes were found on the configured ports."
    )) {
        if (-not $text.Contains($needle)) {
            throw "stop dry-run case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Stopping ",
        "Gemma/rustgpt-lab local test stack stopped."
    )) {
        if ($text.Contains($needle)) {
            throw "stop dry-run case included forbidden stop output: $needle"
        }
    }
}

function Invoke-BuiltInHelpCase {
    Write-Host ""
    Write-Host "safety_case=built_in_help"
    $output = & cmd.exe /c "`"$startBuiltInCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Start rustgpt-lab with the built-in rust-norion backend.",
        "does not start Gemma 12B or mistralrs",
        "State defaults to target\manual-web-lab-service\built-in-lab-state.",
        ".\tools\rustgpt-lab\start-built-in-lab.cmd -StateDir target\manual-web-lab-service\built-in-lab-state",
        ".\tools\rustgpt-lab\start-built-in-lab.cmd -CheckOnly",
        "-LabBackendTimeoutSeconds <n>  Web Lab -> rust-norion total streaming window, default 900",
        "Port map:",
        "7878 = rust-norion built-in backend for safe local UI tests.",
        "8787 = rustgpt-lab Web UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime; this built-in path does not use it."
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in help case missing expected output: $needle"
        }
    }
    if ($text.Contains("scripts\start-built-in-lab.ps1 -CheckOnly")) {
        throw "built-in help case still references direct script CheckOnly path"
    }
}

function Invoke-BuiltInCheckOnlyCase {
    Write-Host ""
    Write-Host "safety_case=built_in_check_only"
    $backendPort = Get-FreeTcpPort
    $webPort = Get-FreeTcpPort
    $stateDir = Join-Path ([System.IO.Path]::GetTempPath()) "rustgpt-lab-built-in-check-only-state"
    $memoryFile = Join-Path $stateDir "memory.ndkv"
    $experienceFile = Join-Path $stateDir "experience.ndkv"
    $adaptiveFile = Join-Path $stateDir "adaptive.ndkv"

    $output = & cmd.exe /c "`"$startBuiltInCmd`" -RepoRoot `"$RepoRoot`" -BackendPort $backendPort -WebPort $webPort -StateDir `"$stateDir`" -SkipBuild -CheckOnly" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in check-only case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Built-in Web Lab safe path",
        "CheckOnly: no build, no process start, no browser open, no .ndkv writes.",
        "StateDir: $stateDir",
        "Lab backend response window: 900s",
        "Memory:   $memoryFile",
        "Experience: $experienceFile",
        "Adaptive: $adaptiveFile",
        "Backend port $backendPort is free.",
        "Web port $webPort is free."
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in check-only case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Building rust-norion",
        "Starting rust-norion",
        "rust-norion pid:",
        "rustgpt-lab pid:"
    )) {
        if ($text.Contains($needle)) {
            throw "built-in check-only case included forbidden startup output: $needle"
        }
    }
}

function Invoke-BuiltInStatusReadOnlyCase {
    Write-Host ""
    Write-Host "safety_case=built_in_status_read_only"
    $backendPort = Get-FreeTcpPort
    $webPort = Get-FreeTcpPort

    $output = & cmd.exe /c "`"$statusBuiltInCmd`" -BackendPort $backendPort -WebPort $webPort" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in status read-only case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Built-in Web Lab status",
        "Read-only check for rust-norion built-in backend + rustgpt-lab Web UI.",
        "No Gemma/mistralrs process is started or queried.",
        "rust-norion",
        "rustgpt-lab"
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in status read-only case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Starting ",
        "Stopping "
    )) {
        if ($text.Contains($needle)) {
            throw "built-in status read-only case included forbidden side-effect output: $needle"
        }
    }
}

function Invoke-BuiltInStopHelpCase {
    Write-Host ""
    Write-Host "safety_case=built_in_stop_help"

    $output = & cmd.exe /c "`"$stopBuiltInCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in stop help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Stop the built-in rust-norion + rustgpt-lab Web Lab.",
        "Use -DryRun to inspect targets first.",
        "DANGER: -ForceAll stops every local process named rust-norion or rustgpt-lab",
        ".\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun",
        "Port map:",
        "7878 = rust-norion built-in backend for safe local UI tests.",
        "8787 = rustgpt-lab Web UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime; this built-in stop path does not target it."
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in stop help case missing expected output: $needle"
        }
    }
}

function Invoke-BuiltInStatusHelpCase {
    Write-Host ""
    Write-Host "safety_case=built_in_status_help"

    $output = & cmd.exe /c "`"$statusBuiltInCmd`" -Help" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in status help case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Show built-in rust-norion + rustgpt-lab Web Lab status.",
        "This is read-only. It does not start Gemma, stop processes, or write .ndkv files.",
        "Port map:",
        "7878 = rust-norion built-in backend for safe local UI tests.",
        "8787 = rustgpt-lab Web UI and local SSE proxy.",
        "8686 = optional Gemma/mistralrs runtime; this built-in status path does not query it."
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in status help case missing expected output: $needle"
        }
    }
}

function Invoke-BuiltInStopDryRunCase {
    Write-Host ""
    Write-Host "safety_case=built_in_stop_dry_run"
    $backendPort = Get-FreeTcpPort
    $webPort = Get-FreeTcpPort

    $output = & cmd.exe /c "`"$stopBuiltInCmd`" -BackendPort $backendPort -WebPort $webPort -DryRun" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "built-in stop dry-run case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "Skipping backend port ${backendPort}: /health did not confirm rust-norion runtime_mode=built-in.",
        "Skipping Web port ${webPort}: /health did not confirm rustgpt-lab backend=127.0.0.1:$backendPort.",
        "No confirmed built-in Web Lab processes were found on the configured ports."
    )) {
        if (-not $text.Contains($needle)) {
            throw "built-in stop dry-run case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Stopping ",
        "Built-in Web Lab stopped."
    )) {
        if ($text.Contains($needle)) {
            throw "built-in stop dry-run case included forbidden stop output: $needle"
        }
    }
}

function Invoke-StartCheckOnlyCase {
    Write-Host ""
    Write-Host "safety_case=start_check_only"
    $mistralPort = Get-FreeTcpPort
    $backendPort = Get-FreeTcpPort
    $labPort = Get-FreeTcpPort
    $stateDir = Join-Path ([System.IO.Path]::GetTempPath()) "rustgpt-lab-check-only-state"
    $memoryFile = Join-Path $stateDir "memory.ndkv"
    $experienceFile = Join-Path $stateDir "experience.ndkv"
    $adaptiveFile = Join-Path $stateDir "adaptive.ndkv"

    $output = & cmd.exe /c "`"$startCmd`" -RepoRoot `"$RepoRoot`" -Snapshot `"$RepoRoot`" -HfCache `"$RepoRoot`" -MistralPort $mistralPort -BackendPort $backendPort -LabPort $labPort -StateDir `"$stateDir`" -MinFreeRamGB 0 -MinFreeGpuGB 0 -CheckOnly" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "start check-only case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "check_only=true",
        "starts_process=false",
        "builds_binaries=false",
        "writes_state=false",
        "experience_safety=isolated_state_dir",
        "snapshot_exists=True",
        "state_dir=$stateDir",
        "memory_file=$memoryFile",
        "experience_file=$experienceFile",
        "adaptive_file=$adaptiveFile"
    )) {
        if (-not $text.Contains($needle)) {
            throw "start check-only case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Starting Gemma 12B runtime",
        "rust-norion pid:",
        "rustgpt-lab pid:"
    )) {
        if ($text.Contains($needle)) {
            throw "start check-only case included forbidden startup output: $needle"
        }
    }
}

function Invoke-StartCheckOnlyProjectStateCase {
    Write-Host ""
    Write-Host "safety_case=start_check_only_project_state"
    $mistralPort = Get-FreeTcpPort
    $backendPort = Get-FreeTcpPort
    $labPort = Get-FreeTcpPort
    $memoryFile = Join-Path $RepoRoot "noiron-memory.ndkv"
    $experienceFile = Join-Path $RepoRoot "noiron-experience.ndkv"
    $adaptiveFile = Join-Path $RepoRoot "noiron-adaptive.ndkv"

    $output = & cmd.exe /c "`"$startCmd`" -RepoRoot `"$RepoRoot`" -Snapshot `"$RepoRoot`" -HfCache `"$RepoRoot`" -MistralPort $mistralPort -BackendPort $backendPort -LabPort $labPort -UseProjectState -MinFreeRamGB 0 -MinFreeGpuGB 0 -CheckOnly" 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "start check-only project-state case failed with exit code $exitCode"
    }
    foreach ($needle in @(
        "check_only=true",
        "starts_process=false",
        "writes_state=false",
        "experience_safety=project_state_requested",
        "memory_file=$memoryFile",
        "experience_file=$experienceFile",
        "adaptive_file=$adaptiveFile",
        "UseProjectState was requested"
    )) {
        if (-not $text.Contains($needle)) {
            throw "start check-only project-state case missing expected output: $needle"
        }
    }
    foreach ($needle in @(
        "Starting Gemma 12B runtime",
        "rust-norion pid:",
        "rustgpt-lab pid:"
    )) {
        if ($text.Contains($needle)) {
            throw "start check-only project-state case included forbidden startup output: $needle"
        }
    }
}

Invoke-HelpCase
Invoke-SafetyHelpCase
Invoke-WebSseParserCase
Invoke-WebUiInteractionCase
Invoke-ReplHelpCase
Invoke-ReplSkipStartBackendMissingCase
Invoke-StartHelpCase
Invoke-StatusHelpCase
Invoke-StopHelpCase
Invoke-StatusReadOnlyCase
Invoke-StopDryRunCase
Invoke-BuiltInHelpCase
Invoke-BuiltInCheckOnlyCase
Invoke-BuiltInStatusHelpCase
Invoke-BuiltInStatusReadOnlyCase
Invoke-BuiltInStopHelpCase
Invoke-BuiltInStopDryRunCase
Invoke-StartCheckOnlyCase
Invoke-StartCheckOnlyProjectStateCase
Invoke-ClientLabUnreachableCase
Invoke-ClientPreflightBlockedCase -Name busy_blocked -HealthBody '{"ok":true,"engine_busy":true,"active_engine_requests":1,"gemma_runtime_reachable":true}' -MustContain @(
    "rust-norion backend is busy"
)
Invoke-ClientPreflightBlockedCase -Name readiness_blocked -HealthBody '{"ok":true,"engine_busy":false,"readiness_ok":false,"readiness_failures":["backend readiness failed offline"],"safe_device_ok":true,"gemma_runtime_reachable":true}' -MustContain @(
    "rust-norion backend prompt gate failed: readiness=false: backend readiness failed offline"
)
Invoke-ClientPreflightBlockedCase -Name safe_device_blocked -HealthBody '{"ok":true,"engine_busy":false,"readiness_ok":true,"safe_device_ok":false,"safe_device_failures":["GPU memory below threshold"],"gemma_runtime_reachable":true}' -MustContain @(
    "rust-norion backend prompt gate failed: safe-device=false: GPU memory below threshold"
)
Invoke-ClientPreflightBlockedCase -Name experience_blocked -HealthBody '{"ok":true,"engine_busy":false,"readiness_ok":true,"safe_device_ok":true,"gemma_runtime_reachable":true,"experience_hygiene":{"clean":false,"quarantine_candidates":1,"repairable_legacy_metadata_lessons":0,"repairable_index_records":0,"index":{"retrieval_ready":false,"risk_level":"blocked"}}}' -MustContain @(
    "rust-norion backend prompt gate failed: experience_hygiene.clean=false"
)
Invoke-ClientPreflightBlockedCase -Name gemma_runtime_unreachable -HealthBody '{"ok":true,"engine_busy":false,"readiness_ok":true,"safe_device_ok":true,"runtime_mode":"gemma-http","gemma_runtime_server":"http://127.0.0.1:1","gemma_runtime_reachable":false}' -MustContain @(
    "Gemma runtime is configured but not reachable at http://127.0.0.1:1",
    "Inspect with tools\rustgpt-lab\status-gemma-lab.cmd",
    "start-gemma-lab.cmd -CheckOnly",
    "only run start-gemma-lab.cmd without -CheckOnly when you intentionally want to start the Gemma stack",
    "Do not send prompts to 8686 directly"
) -MustNotContain @(
    "Run tools\rustgpt-lab\start-gemma-lab.cmd first."
)
Invoke-ClientCase -Name success -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "hello from fake lab",
    "[final answer]",
    "fake final",
    "[DONE]"
)
Invoke-ClientCase -Name heartbeat -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "[heartbeat] waiting on fake backend",
    "after heartbeat",
    "heartbeat final",
    "[DONE]"
)
Invoke-ClientCase -Name comment_only -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "[DONE]"
) -MustNotContain @(
    "keep-alive",
    "[message]"
)
Invoke-ClientCase -Name cr_only_frames -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "cr only",
    "[DONE]"
)
Invoke-ClientCase -Name multiline_data -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "line one",
    "line two",
    "[DONE]"
)
Invoke-ClientCase -Name empty_event_field -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "[message] empty event became message",
    "[DONE]"
)
Invoke-ClientCase -Name no_colon_fields -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "[message] no-colon event became message",
    "[status]",
    "[DONE]"
)
Invoke-ClientCase -Name field_value_spacing -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "  indented",
    "[DONE]"
)
Invoke-ClientCase -Name business_cycle_payload -TimeoutSeconds 5 -ExpectSuccess $true -MustContain @(
    "[stage] business-cycle payload accepted",
    "[final] business_cycle passed=True",
    "[final] feedback_applied=True rust_check_passed=True self_improve_passed=False",
    "business payload ok",
    "[DONE]"
) -ExtraArgs @(
    "-Endpoint business-cycle",
    "-Output enhanced",
    "-Profile review",
    "-NoSelfImprove",
    "-FeedbackAmount 0.75",
    "-RustCheckCode `"pub fn ok() -> bool { true }`""
)
Invoke-ClientCase -Name error -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "[error] blocked by fake gate",
    "[DONE]",
    "lab chat stream returned SSE error: blocked by fake gate"
) -MustNotContain @(
    "A task was canceled",
    "At $clientScript"
)
Invoke-ClientCase -Name error_without_done -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "[error] backend closed after error",
    "lab chat stream returned SSE error: backend closed after error"
) -MustNotContain @(
    "lab chat stream truncated",
    "At $clientScript"
)
Invoke-ClientCase -Name http_error -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "chat stream request failed with HTTP 503: fake upstream unavailable"
) -MustNotContain @(
    "lab chat stream truncated",
    "[DONE]",
    "At $clientScript"
)
Invoke-ClientCase -Name truncated -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "partial answer",
    "lab chat stream truncated: EOF before done event"
) -MustNotContain @(
    "One or more errors occurred",
    "At $clientScript"
)
Invoke-ClientCase -Name final_without_done -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "final before eof",
    "lab chat stream truncated: EOF before done event"
) -MustNotContain @(
    "[DONE]",
    "One or more errors occurred",
    "At $clientScript"
)
Invoke-ClientCase -Name incomplete_frame -TimeoutSeconds 5 -ExpectSuccess $false -MustContain @(
    "lab chat stream truncated: incomplete SSE frame before EOF"
) -MustNotContain @(
    "partial frame without separator",
    "One or more errors occurred",
    "At $clientScript"
)
Invoke-ClientCase -Name timeout -TimeoutSeconds 1 -ExpectSuccess $false -MustContain @(
    "[status] waiting forever",
    "lab chat stream timed out after 1s"
) -MustNotContain @(
    "A task was canceled",
    "At $clientScript"
)
Invoke-ClientCase -Name no_headers_timeout -TimeoutSeconds 1 -ExpectSuccess $false -MustContain @(
    "lab chat stream timed out after 1s"
) -MustNotContain @(
    "A task was canceled",
    "At $clientScript"
)

Write-Host ""
Write-Host "rustgpt-lab safety tests passed"
