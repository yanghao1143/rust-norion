pub(crate) fn usage() -> &'static str {
    "SmartSteam Forge\n\
     \n\
     Usage:\n\
       smartsteam-forge [--backend 127.0.0.1:7878] [--mock]\n\
       smartsteam-forge --mock --prompt \"hello\"\n\
       smartsteam-forge --backend 127.0.0.1:7878 --mode chat --prompt \"hello\"\n\
     \n\
     Startup helpers:\n\
       cargo run -- --backend 127.0.0.1:7878 connects only; it does not start rust-norion\n\
       .\\start-forge-stack.cmd starts built-in rust-norion + optional web lab + TUI; no Gemma\n\
       .\\smoke-forge-stack.cmd runs a non-Gemma backend + Forge integration smoke\n\
       .\\start-forge-ui.cmd attaches to an already running backend\n\
       .\\start-gemma-forge.cmd explicitly starts the Gemma 12B full stack\n\
       .\\start-remote-gemma-chain.cmd starts local Rust control plane against a remote Mac model box\n\
       .\\start-remote-gemma-forge.cmd -CheckOnly preflights remote Gemma + backend + Web Lab + Forge\n\
       .\\start-remote-gemma-forge.cmd starts one 12B quality worker plus local backend/Web Lab/TUI\n\
       .\\status-remote-gemma-chain.cmd checks remote model + tunnel + local backend/Web Lab\n\
       .\\smoke-remote-gemma-chain.cmd validates remote model + backend + Web Lab SSE + Forge CLI\n\
       .\\run-remote-gemma-evolution-loop.cmd -CheckOnly preflights the evolution-loop handoff\n\
       .\\evolution-daemon.cmd -StartCheck previews unattended daemon startup through Forge\n\
       .\\evolution-daemon.cmd -JsonStartCheck prints machine-readable StartCheck JSON; never starts the daemon\n\
      .\\evolution-daemon.cmd -Start starts the budgeted unattended evolution daemon\n\
      .\\evolution-daemon.cmd -Status shows daemon + ledger + report + model-pool state\n\
      cargo run -- --evolution-strict-summary shows compact strict unattended status artifact\n\
      .\\evolution-daemon.cmd -Candidates shows recent evolution backlog candidates\n\
       .\\evolution-daemon.cmd -Candidates -CandidatesSave appends deduped candidates to evolution-candidates.jsonl\n\
       .\\evolution-daemon.cmd -CandidateList -CandidateStatus accepted lists accepted backlog items\n\
       .\\evolution-daemon.cmd -CandidateGate checks candidate lifecycle readiness with a nonzero failure exit\n\
       .\\evolution-daemon.cmd -CandidateApplyCheck <id|next> previews accepted candidate implementation checks\n\
       .\\evolution-daemon.cmd -CandidateValidate <id> records append-only validation evidence\n\
       .\\evolution-daemon.cmd -CandidateMark <id> -CandidateStatus accepted appends a candidate status audit event\n\
       .\\evolution-daemon.cmd -Watch -Count 3 watches daemon progress without sending prompts\n\
       .\\evolution-daemon.cmd -Stop stops the selected daemon work dir\n\
       Recommended pool topology: one Gemma 12B quality worker plus small helper workers; do not run multiple 12B workers on Apple unified memory/GPU\n\
     \n\
     Options:\n\
       --backend <host:port>        rust-norion backend, http(s) prefix is accepted\n\
       --mock                       use offline mock provider\n\
       --prompt <text>              run one non-interactive prompt and exit\n\
       --once <text>                alias for --prompt\n\
       --smoke                      run a built-in one-shot smoke prompt\n\
       --mode <mode>                chat, generate, or business-cycle\n\
       --context-messages <count>   max short-context messages including next prompt, default 64\n\
       --context-window <count>     alias for --context-messages\n\
       --max-context-messages <n>   alias for --context-messages\n\
       --max-tokens <count|default> per-request output token budget sent to rust-norion, default 262144\n\
       --max-output-tokens <value>  alias for --max-tokens\n\
       --health, --check            check backend /health without running inference; shows busy/device/runtime\n\
       --hygiene                    show backend experience hygiene report without inference\n\
       --hygiene-quarantine         dry-run experience hygiene quarantine; never applies changes\n\
       --hygiene-limit <count>      hygiene finding display limit, default 20\n\
       --repair, --repair-dry-run   dry-run legacy metadata repair; never applies changes\n\
       --repair-limit <count>       repair preview display limit, default 20\n\
       --audit, --cleanup-audit     show hygiene + quarantine/repair dry-runs without inference\n\
       --audit-limit <count>        audit sample display limit, default 20\n\
       --pool-status                show read-only backend model-pool status\n\
       --pool-manifest              show read-only backend model-pool manifest and worker plan\n\
       --pool-advice                explain whether Apple model-pool expansion is safe; read-only\n\
       --pool-smoke                 run status + advice + helper route plans; read-only, no prompt\n\
       --pool-watch [seconds]       repeatedly show read-only model-pool status, default 5 seconds\n\
       --pool-watch-count <count>   stop --pool-watch after count iterations, useful for smoke checks\n\
       --pool-route <kind>          show read-only route plan for auto|summary|review|test-gate|index|quality\n\
       --pool-call <kind>           send --prompt to the routed worker after route-plan allows it\n\
      --evolution-status           show read-only evolution-loop daemon, ledger, report, and pool readiness\n\
      --evolution-status-json      print enriched read-only Forge status JSON with candidate start gate\n\
      --evolution-strict-summary   read compact strict unattended status artifact; no script, process, or prompt\n\
      --evolution-strict-summary-json wrap compact strict status artifact in Forge read-only JSON\n\
      --evolution-strict-summary-path <path> custom strict summary artifact path\n\
      --evolution-watch [seconds]  repeatedly show daemon status; read-only, default 5 seconds\n\
       --evolution-watch-count <n>  stop --evolution-watch after n iterations\n\
       --evolution-candidates       show recent model-produced improvement candidates; read-only\n\
       --evolution-candidate-list   list persisted evolution-candidates.jsonl backlog; read-only\n\
       --evolution-candidate-gate   fail unless accepted candidates are cleared and implemented candidates are validated\n\
       --evolution-candidates-limit <n> ledger fallback candidate count, default 5\n\
       --evolution-candidates-save  append deduped candidates to work-dir evolution-candidates.jsonl\n\
       --evolution-candidates-backlog <path> custom JSONL backlog path; implies save\n\
       --evolution-candidate-mark <id> append a status audit event for a backlog candidate\n\
       --evolution-candidate-apply-check <id|next> dry-run accepted candidate implementation readiness; read-only\n\
       --evolution-candidate-validate <id> append validation evidence for a backlog candidate\n\
       --evolution-candidate-validation-command <cmd> validation command text to record with validate\n\
       --evolution-candidate-validation-status <code> validation process exit code, 0 means passed\n\
       --evolution-candidate-status <status> new, accepted, implemented, or rejected; required with mark, optional filter with list\n\
       --evolution-candidate-note <text> optional note for the status audit event\n\
       --evolution-start            start the budgeted unattended evolution daemon after candidate preflight; sends prompts by design\n\
       --evolution-stop             stop the unattended evolution daemon for the selected work dir\n\
       --evolution-check-only       with start/stop, print the daemon action without starting/stopping anything\n\
       --evolution-start-check      alias for --evolution-start --evolution-check-only\n\
       --evolution-start-check-json print machine-readable start preflight + command preview; read-only\n\
       --evolution-stop-check       alias for --evolution-stop --evolution-check-only\n\
       --evolution-work-dir <dir>   daemon work dir, default target\\evolution\\daemon\n\
       --evolution-interval-secs <n> start-loop sleep interval between rounds\n\
       --evolution-max-tokens <n> per-round evolution generation budget\n\
       --evolution-max-total-tokens <n> total runtime-token budget, 0 disables\n\
       --evolution-max-runtime-secs <n> total observed runtime budget, 0 disables\n\
       --evolution-max-failures <n> stop after consecutive failures\n\
       --evolution-max-no-feedback-rounds <n> stop after rounds without feedback, 0 disables\n\
       --evolution-timeout-secs <n> backend request timeout for daemon rounds\n\
       TUI /context, /ctx           preview context and context_budget without inference\n\
       TUI /context-window <count>  set short-context window; aliases /context-messages, /ctx-window\n\
       TUI /max-tokens <count>      set request max_tokens; use default/off to use backend default\n\
       TUI /retrieve [limit] <text> previews experience retrieval without inference\n\
       TUI /index-notes           shows model-pool index notes and marks the active index context\n\
       TUI /index-notes clear     clears all model-pool index blocks, including legacy tails\n\
       TUI /pool-status             shows model-pool status without launching workers or sending prompts\n\
       TUI /pool-manifest           shows model-pool manifest and planned worker shape; read-only\n\
       TUI /pool-advice             shows Apple model-pool expansion advice; read-only\n\
       TUI /pool-smoke              checks status, advice, and helper routes; read-only\n\
       TUI /pool-watch [sec] [n]    nonblocking model-pool status watch; /pool-watch off stops it\n\
       TUI /pool-route <kind>       shows route plan only; it does not dispatch the task\n\
       TUI /pool-call <kind> <text> sends text to the selected model-pool worker\n\
       TUI /strict-status [path]    shows strict unattended evolution status artifact; read-only\n\
       --doctor, --diagnose         print health, readiness, safe-device, and Chinese diagnostic commands\n\
       --preflight, --ready         check readiness and exit without running inference\n\
       --sessions [filter]          list recorded transcripts without contacting backend\n\
       --summary [index|id]         write a deterministic markdown session summary\n\
       --session-limit <count>      transcript list limit, default 50\n\
       --require-health             check backend readiness before one-shot or TUI startup\n\
       --require-safe-device        also block Gemma 12B CPU/disk-first / non-GPU-first preflight warnings\n\
       --connect-timeout-ms <ms>    backend connect timeout, default 5000\n\
       --read-timeout-ms <ms>       per-read poll/heartbeat interval, not total stream timeout, default 2000\n\
       --timeout-secs <seconds>     total one-shot/stream backend request timeout, default 900\n\
       --request-timeout-secs <s>   alias for --timeout-secs; total request window, not read polling\n\
       -h, --help                   show this help"
}
