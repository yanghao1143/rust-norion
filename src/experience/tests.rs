use super::*;
use crate::adaptive_state::LiveInferenceEvolution;
use crate::disk_kv::DiskKvStore;
use crate::gist_memory::{GistLevel, GistRecord};
use crate::hierarchy::HierarchyWeights;
use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
use crate::reflection::{ReflectionIssue, ReflectionSeverity, RuntimeDiagnostics};
use crate::router::RouteBudget;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn records_and_ranks_lessons() {
    let mut store = ExperienceStore::new();
    store.record(input("weak", 0.2));
    store.record(input("strong", 0.9));

    let lessons = store.top_lessons(0.5, 4);

    assert_eq!(lessons.len(), 1);
    assert_eq!(lessons[0].lesson, "strong");
}

#[test]
fn retrieves_relevant_lessons() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Rust adaptive router".to_owned(),
        lesson: "prefer token-window feedback for router stability".to_owned(),
        ..input("router", 0.9)
    });
    store.record(ExperienceInput {
        prompt: "long form story writing".to_owned(),
        profile: TaskProfile::Writing,
        lesson: "prefer global continuity".to_owned(),
        ..input("writing", 0.9)
    });

    let matches = store.retrieve_lessons("Rust router feedback", TaskProfile::Coding, 2);

    assert!(!matches.is_empty());
    assert!(matches[0].lesson.contains("router"));
}

#[test]
fn record_mut_updates_experience_notes_for_replay() {
    let path = temp_path("experience-feedback-note");
    let mut store = ExperienceStore::new();
    let id = store.record(input("external feedback", 0.87));

    store
            .record_mut(id)
            .unwrap()
            .process_reward
            .notes
            .insert(
                0,
                "memory_feedback:external:reinforced=2:penalized=0:reinforcement_amount=1.000000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.250000"
                    .to_owned(),
            );
    store.save_to_disk_kv(&path).unwrap();
    let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

    let note = loaded.records()[0]
        .process_reward
        .notes
        .iter()
        .find(|note| note.starts_with("memory_feedback:external:"))
        .unwrap();
    assert!(note.contains("applied=2"));
    assert!(note.contains("strength_delta=0.250000"));
    cleanup(path);
}

#[test]
fn disk_kv_roundtrip_preserves_experience() {
    let path = temp_path("experience");
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        gist_records: vec![gist("document", GistLevel::Document, 0.88)],
        gist_memory_ids: vec![7, 8],
        ..input("stored", 0.87)
    });

    store.save_to_disk_kv(&path).unwrap();
    let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.records()[0].id, id);
    assert_eq!(loaded.records()[0].lesson, "stored");
    assert_eq!(loaded.records()[0].profile, TaskProfile::Coding);
    assert_eq!(loaded.records()[0].gist_records.len(), 1);
    assert_eq!(loaded.records()[0].gist_memory_ids, vec![7, 8]);
    assert_eq!(loaded.records()[0].used_memory_ids, vec![3, 5]);
    assert_eq!(loaded.records()[0].stored_runtime_kv_memory_ids, vec![11]);
    assert!(
        loaded.records()[0]
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("memory_feedback:"))
    );
    assert_eq!(
        loaded.records()[0].runtime_diagnostics.model_id.as_deref(),
        Some("noiron-test-runtime")
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .selected_adapter
            .as_deref(),
        Some("portable-rust")
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .device_profile
            .as_deref(),
        Some("cpu")
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .primary_lane
            .as_deref(),
        Some("cpu-vector")
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .fallback_lane
            .as_deref(),
        Some("cpu-portable")
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .memory_mode
            .as_deref(),
        Some("tiered-disk")
    );
    assert_eq!(loaded.records()[0].runtime_diagnostics.layer_count, 8);
    assert_eq!(
        loaded.records()[0].runtime_diagnostics.forward_energy,
        Some(0.25)
    );
    assert_eq!(
        loaded.records()[0].runtime_diagnostics.kv_influence,
        Some(0.75)
    );
    assert_eq!(loaded.records()[0].runtime_token_metrics.token_count, 3);
    assert_eq!(loaded.records()[0].runtime_token_metrics.entropy_count, 3);
    assert_eq!(loaded.records()[0].runtime_token_metrics.logprob_count, 2);
    assert_eq!(
        loaded.records()[0].runtime_token_metrics.average_entropy,
        Some(0.42)
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_token_metrics
            .average_neg_logprob,
        Some(0.70)
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_token_metrics
            .uncertainty_perplexity,
        Some(4.38)
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .hot_kv_precision_bits,
        Some(8)
    );
    assert_eq!(
        loaded.records()[0]
            .runtime_diagnostics
            .cold_kv_precision_bits,
        Some(4)
    );
    assert!(
        loaded.records()[0]
            .runtime_diagnostics
            .has_valid_kv_precision_signal()
    );
    assert_eq!(loaded.records()[0].reflection_issues.len(), 1);
    assert_eq!(
        loaded.records()[0].reflection_issues[0].severity,
        ReflectionSeverity::Warning
    );
    assert_eq!(
        loaded.records()[0].revision_actions,
        vec!["revise_reflection_signal".to_owned()]
    );
    assert!((loaded.records()[0].route_budget.attention_fraction - 0.4).abs() < 0.0001);
    assert!((loaded.records()[0].process_reward.total - 0.5).abs() < 0.0001);
    assert!((loaded.records()[0].live_evolution.router_threshold_delta - 0.030000).abs() < 0.0001);
    assert!((loaded.records()[0].live_evolution.hierarchy_weight_delta - 0.040000).abs() < 0.0001);
    assert_eq!(
        loaded.records()[0].live_evolution.online_reward_feedbacks,
        1
    );
    assert_eq!(
        loaded.records()[0]
            .live_evolution
            .online_reward_reinforcements,
        1
    );
    assert_eq!(
        loaded.records()[0].live_evolution.online_reward_penalties,
        0
    );
    assert!((loaded.records()[0].live_evolution.online_reward_strength - 0.72).abs() < 0.0001);
    assert!(
        (loaded.records()[0]
            .live_evolution
            .online_reward_reinforcement_strength
            - 0.72)
            .abs()
            < 0.0001
    );
    assert_eq!(
        loaded.records()[0]
            .live_evolution
            .online_reward_penalty_strength,
        0.0
    );
    assert_eq!(loaded.records()[0].live_evolution.memory_reinforcements, 1);
    assert_eq!(loaded.records()[0].live_evolution.memory_penalties, 0);
    assert!(loaded.records()[0].live_evolution.stored_memory);
    assert_eq!(loaded.records()[0].live_evolution.stored_gist_memories, 2);
    assert_eq!(
        loaded.records()[0]
            .live_evolution
            .stored_runtime_kv_memories,
        1
    );
    assert_eq!(loaded.records()[0].live_evolution.reflection_issues, 1);
    assert_eq!(
        loaded.records()[0]
            .live_evolution
            .critical_reflection_issues,
        0
    );
    assert_eq!(loaded.records()[0].live_evolution.revision_actions, 1);
    cleanup(path);
}

#[test]
fn record_and_load_drop_untrusted_runtime_selected_adapter() {
    let path = temp_path("experience-runtime-adapter-sanitize");
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("noiron-test-runtime".to_owned()),
            selected_adapter: Some("unknown-adapter secret=sk-experience-leak".to_owned()),
            forward_energy: Some(0.25),
            kv_influence: Some(0.75),
            ..RuntimeDiagnostics::default()
        },
        ..input("runtime adapter sanitize", 0.91)
    });

    assert_eq!(
        store.records()[0].runtime_diagnostics.selected_adapter,
        None
    );

    store.save_to_disk_kv(&path).unwrap();
    let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded.records()[0].runtime_diagnostics.selected_adapter,
        None
    );
    cleanup(path);
}

#[test]
fn save_to_disk_kv_drops_mutated_untrusted_runtime_selected_adapter_bytes() {
    let path = temp_path("experience-runtime-adapter-mutated-save-sanitize");
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("noiron-test-runtime".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.25),
            kv_influence: Some(0.75),
            ..RuntimeDiagnostics::default()
        },
        ..input("runtime adapter save sanitize", 0.91)
    });
    store
        .record_mut(id)
        .unwrap()
        .runtime_diagnostics
        .selected_adapter = Some("unknown-adapter secret=sk-mutated-experience".to_owned());

    store.save_to_disk_kv(&path).unwrap();
    let disk = DiskKvStore::open_read_only_existing(&path)
        .unwrap()
        .expect("experience disk kv");
    let encoded = String::from_utf8(disk.get(&format!("experience/{id}")).unwrap().unwrap())
        .expect("experience record utf8");
    let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

    assert!(!encoded.contains("unknown-adapter"));
    assert!(!encoded.contains("secret="));
    assert!(!encoded.contains("sk-mutated-experience"));
    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded.records()[0].runtime_diagnostics.selected_adapter,
        None
    );
    cleanup(path);
}

#[test]
fn legacy_live_evolution_without_online_reward_feedback_loads_defaults() {
    let loaded = deserialize_live_evolution("0.030000,0.040000,1,0,1,2,1,1,0,1").unwrap();

    assert_eq!(loaded.online_reward_feedbacks, 0);
    assert_eq!(loaded.online_reward_reinforcements, 0);
    assert_eq!(loaded.online_reward_penalties, 0);
    assert_eq!(loaded.online_reward_strength, 0.0);
    assert_eq!(loaded.online_reward_reinforcement_strength, 0.0);
    assert_eq!(loaded.online_reward_penalty_strength, 0.0);
    assert_eq!(loaded.memory_reinforcements, 1);
    assert_eq!(loaded.stored_gist_memories, 2);
    assert_eq!(loaded.revision_actions, 1);
}

#[test]
fn legacy_live_evolution_without_online_reward_strength_loads_defaults() {
    let loaded = deserialize_live_evolution("0.030000,0.040000,1,1,0,1,0,1,2,1,1,0,1").unwrap();

    assert_eq!(loaded.online_reward_feedbacks, 1);
    assert_eq!(loaded.online_reward_reinforcements, 1);
    assert_eq!(loaded.online_reward_penalties, 0);
    assert_eq!(loaded.online_reward_strength, 0.0);
    assert_eq!(loaded.online_reward_reinforcement_strength, 0.0);
    assert_eq!(loaded.online_reward_penalty_strength, 0.0);
    assert_eq!(loaded.memory_reinforcements, 1);
    assert_eq!(loaded.stored_runtime_kv_memories, 1);
    assert_eq!(loaded.revision_actions, 1);
}

#[test]
fn deserialize_live_evolution_rejects_online_reward_feedback_count_mismatch() {
    assert!(
        deserialize_live_evolution(
            "0.030000,0.040000,2,1,0,0.720000,0.720000,0.000000,1,0,1,2,1,1,0,1"
        )
        .is_none()
    );
}

#[test]
fn deserialize_live_evolution_rejects_online_reward_strength_total_mismatch() {
    assert!(
        deserialize_live_evolution(
            "0.030000,0.040000,2,1,1,0.720000,0.720000,0.250000,1,0,1,2,1,1,0,1"
        )
        .is_none()
    );
}

#[test]
fn deserialize_live_evolution_rejects_feedback_without_strength_when_strength_fields_present() {
    assert!(
        deserialize_live_evolution(
            "0.030000,0.040000,1,1,0,0.000000,0.000000,0.000000,1,0,1,2,1,1,0,1"
        )
        .is_none()
    );
}

#[test]
fn deserialize_live_evolution_rejects_component_strength_without_count() {
    assert!(
        deserialize_live_evolution(
            "0.030000,0.040000,1,1,0,0.920000,0.720000,0.200000,1,0,1,2,1,1,0,1"
        )
        .is_none()
    );
}

#[test]
fn deserialize_record_rejects_malformed_live_evolution_field() {
    let mut store = ExperienceStore::new();
    store.record(input("malformed live evolution", 0.82));
    let current = serialize_record(&store.records()[0]);
    let mut fields = current.split('\t').map(str::to_owned).collect::<Vec<_>>();
    fields[21] = escape_field("0.030000,0.040000,1,1,0,0.000000,0.000000,0.000000,1,0,1,2,1,1,0,1");
    let malformed = fields.join("\t");

    assert!(deserialize_record(&malformed).is_none());
}

#[test]
fn legacy_experience_records_without_runtime_token_metrics_load_defaults() {
    let mut store = ExperienceStore::new();
    store.record(input("legacy", 0.82));
    let current = serialize_record(&store.records()[0]);
    let legacy = current
        .rsplit_once('\t')
        .map(|(legacy, _)| legacy)
        .unwrap_or(&current);

    let loaded = deserialize_record(legacy).unwrap();

    assert_eq!(loaded.lesson, "legacy");
    assert_eq!(
        loaded.runtime_token_metrics,
        ExperienceRuntimeTokenMetrics::default()
    );
    assert!(!loaded.runtime_token_metrics.has_uncertainty_signal());
}

#[test]
fn retrieve_lessons_includes_gist_hints() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "long context scheduler".to_owned(),
        lesson: "reuse recursive chunk summaries".to_owned(),
        gist_records: vec![gist(
            "recursive chunks preserve overlap",
            GistLevel::Section,
            0.91,
        )],
        ..input("gist", 0.9)
    });

    let matches = store.retrieve_lessons("recursive overlap", TaskProfile::Coding, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].gist_hints.len(), 1);
    assert!(matches[0].gist_hints[0].contains("recursive chunks"));
    assert_eq!(matches[0].reward_action, RewardAction::Hold);
}

#[test]
fn retrieval_uses_reflection_issue_text_but_penalizes_severity() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "generic prompt".to_owned(),
        lesson: "avoid repeating weak answers".to_owned(),
        quality: 0.86,
        reflection_issues: vec![ReflectionIssue::new(
            "repetitive_answer",
            ReflectionSeverity::Warning,
            "deduplicate repeated phrases",
        )],
        revision_actions: vec!["deduplicate_repeated_phrases".to_owned()],
        ..input("issue", 0.86)
    });

    let matches = store.retrieve_lessons("deduplicate repeated phrases", TaskProfile::Coding, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].reflection_issue_codes,
        vec!["repetitive_answer".to_owned()]
    );
    assert_eq!(
        matches[0].revision_actions,
        vec!["deduplicate_repeated_phrases".to_owned()]
    );

    let hint = render_experience_hint(&matches[0]);
    assert!(hint.contains("reflection_issues=repetitive_answer"));
    assert!(hint.contains("revision_actions=deduplicate_repeated_phrases"));
}

#[test]
fn retrieval_skips_cross_task_shell_transcript_pollution() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: Rust for-loop answer\nuser: Bash command\nssh -o ConnectTimeout=8 rust-saas-dev 'bash -s' <<'REMOTE'\nTK=\"${PRODUCT_AUTOMATION_TOKEN}\"; API=http://gitlab.local/api/v4\napi \"groups/rust-saas/merge_requests?state=opened\"\nassistant: unrelated shell explanation".to_owned(),
        lesson: "polluted Rust answer mixed with shell merge_requests context".to_owned(),
        quality: 0.99,
        ..input("polluted", 0.99)
    });
    store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10".to_owned(),
        quality: 0.82,
        ..input("clean rust", 0.82)
    });

    let matches = store.retrieve_lessons("帮我用rust输出一段for循环代码", TaskProfile::Coding, 2);

    assert!(!matches.is_empty());
    assert!(matches[0].lesson.contains("clean Rust range loop"));
    assert!(
        matches
            .iter()
            .all(|lesson| !lesson.lesson.contains("merge_requests"))
    );
}

#[test]
fn retrieval_report_counts_cross_task_pollution_skips() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: Rust for-loop answer\nuser: Bash command\nssh -o ConnectTimeout=8 rust-saas-dev 'bash -s'\nAPI=http://gitlab.local/api/v4 merge_requests"
            .to_owned(),
        lesson: "polluted Rust answer mixed with shell merge_requests context".to_owned(),
        quality: 0.99,
        ..input("polluted", 0.99)
    });
    let clean_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10".to_owned(),
        quality: 0.82,
        ..input("clean rust", 0.82)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert_eq!(report.total_records, 2);
    assert_eq!(report.requested_limit, 5);
    assert_eq!(report.skipped_cross_task_pollution, 1);
    assert_eq!(report.retrieval_noise_penalized_candidates, 0);
    assert_eq!(report.retrieval_noise_filtered_candidates, 0);
    assert_eq!(report.suppressed_prompt_index_candidates, 0);
    assert_eq!(report.max_retrieval_noise_penalty, 0.0);
    assert!(report.has_matches());
    assert_eq!(report.matches[0].id, clean_id);
    assert!(report.matches.iter().all(|item| item.id != polluted_id));
}

#[test]
fn retrieval_report_blocks_development_polluted_evidence_from_matches() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "development_evidence_contamination raw poisoned transcript must not retrieve"
            .to_owned(),
        quality: 0.99,
        ..input("polluted evidence", 0.99)
    });
    let clean_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10".to_owned(),
        quality: 0.82,
        ..input("clean rust", 0.82)
    });

    let report = store.retrieval_report("Rust for loop examples", TaskProfile::Coding, 5);

    assert_eq!(report.total_records, 2);
    assert_eq!(report.development_evidence_surface_blocked_candidates, 1);
    assert_eq!(report.skipped_cross_task_pollution, 0);
    assert!(report.has_matches());
    assert_eq!(report.matches[0].id, clean_id);
    assert!(report.matches.iter().all(|item| item.id != polluted_id));
    assert!(
        report
            .matches
            .iter()
            .all(|item| !item.lesson.contains("raw poisoned transcript"))
    );
}

#[test]
fn long_experience_index_is_bounded_and_marked() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: format!(
            "Conversation transcript:\nuser: {}\nassistant: done",
            "rust adaptive router ".repeat(700)
        ),
        lesson: "prefer compact indexed lessons over replaying entire transcripts ".repeat(180),
        ..input("long indexed", 0.9)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let document = super::index::record_index_document(record);
    let report = store.index_report(8);

    assert!(document.compacted);
    assert!(document.text.contains("index_compacted"));
    assert!(document.text.contains("index_sketch:"));
    assert!(document.noise_penalty > 0.0);
    assert!(document.text.chars().count() < 2_600);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 1);
    assert_eq!(
        report.max_record_chars,
        record.prompt.chars().count() + record.lesson.chars().count()
    );
    assert!(
        record
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("experience_index:compacted=true"))
    );
}

#[test]
fn moderate_compacted_transcript_without_gist_is_not_index_noise() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: format!(
            "Conversation transcript:\nuser: {}\nassistant: done",
            "rust local Gemma integration ".repeat(42)
        ),
        lesson: "keep the answer concise and reusable".to_owned(),
        ..input("moderate indexed", 0.88)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let document = super::index::record_index_document(record);
    let report = store.index_report(8);

    assert!(record.prompt.chars().count() > 960);
    assert!(record.prompt.chars().count() < 2_400);
    assert!(document.compacted);
    assert_eq!(document.noise_penalty, 0.0);
    assert_eq!(report.compacted_record_count, 1);
    assert_eq!(report.overlong_record_count, 0);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert!(report.max_record_chars < 2_400, "{report:?}");
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.quality_score >= 0.92, "{report:?}");
    assert!(report.retrieval_ready);
    assert_eq!(report.risk_level, "clean");
    assert_eq!(report.recommended_action, "ready_for_retrieval");
    assert!(report.findings.is_empty());
}

#[test]
fn long_single_document_without_clean_gist_is_index_risk() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: "Gemma model pool routing note ".repeat(360),
        lesson: "keep long raw documents out of the searchable lesson index unless a compact gist exists"
            .to_owned(),
        quality: 0.93,
        ..input("long document without gist", 0.93)
    });

    let report = store.index_report(8);

    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 1);
    assert!(report.max_record_chars > 2_400, "{report:?}");
    assert_eq!(report.noisy_record_count, 1);
    assert_eq!(report.risk_level, "blocked");
    assert_eq!(report.recommended_action, "pause_chat_and_add_clean_gists");
    assert!(report.findings.iter().any(|finding| {
        finding.experience_id == id
            && finding.reason == "overlong_single_document_without_clean_gist"
    }));
}

#[test]
fn long_single_document_with_clean_gist_keeps_index_ready() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: "Gemma model pool routing note ".repeat(360),
        lesson: "reuse compact route evidence instead of replaying raw documents".to_owned(),
        gist_records: vec![gist(
            "Reuse compact Gemma route evidence for model pool scheduling decisions.",
            GistLevel::Document,
            0.93,
        )],
        quality: 0.93,
        ..input("long document with gist", 0.93)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let document = super::index::record_index_document(record);
    let report = store.index_report(8);

    assert!(document.compacted);
    assert_eq!(document.noise_penalty, 0.0);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.quality_score >= 0.92, "{report:?}");
    assert!(report.retrieval_ready);
    assert_eq!(report.risk_level, "clean");
    assert_eq!(report.recommended_action, "ready_for_retrieval");
    assert!(
        record.process_reward.notes.iter().any(|note| {
            note.contains("overlong=true") && note.contains("overlong_without_clean_gist=false")
        }),
        "{:?}",
        record.process_reward.notes
    );
}

#[test]
fn long_generated_response_gets_clean_gist_on_admission() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: "SmartSteam remote helper smoke context ".repeat(360),
        lesson: "Revise_Response: assistant: AsThought: 增加对 test-gate 回滚或重试机制的显式测试用例，避免远程模型池在上下文溢出后静默退化。 Reflection repair: keep the route evidence compact before retrying"
            .to_owned(),
        quality: 0.93,
        ..input("long generated response", 0.93)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let report = store.index_report(8);

    assert!(
        record
            .gist_records
            .iter()
            .any(|gist| gist.title == "Generated response clean gist"
                && gist.summary.contains("test-gate")
                && !gist.summary.contains("Reflection repair:")
                && !gist.summary.contains("[Reflection")
                && !gist.summary.contains("[reflection")
                && !gist.summary.contains("AsThought")
                && !gist.summary.contains("assistant:")),
        "{:?}",
        record.gist_records
    );
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.quality_score >= 0.92, "{report:?}");
    assert!(
        record
            .process_reward
            .notes
            .iter()
            .any(|note| { note.starts_with("experience_index:generated_response_clean_gist") })
    );
}

#[test]
fn flexible_generated_response_clean_gist_note_stays_idempotent_on_admission() {
    let mut store = ExperienceStore::new();
    let existing_note = "Experience_Index ： Generated_Response_Clean_Gist".to_owned();
    let id = store.record(ExperienceInput {
        prompt: "SmartSteam remote helper smoke context ".repeat(360),
        lesson: "Revise_Response: assistant: AsThought: 增加对 test-gate 回滚或重试机制的显式测试用例，避免远程模型池在上下文溢出后静默退化。 Reflection repair: keep the route evidence compact before retrying"
            .to_owned(),
        quality: 0.93,
        process_reward: ProcessRewardReport {
            total: 0.93,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![existing_note.clone()],
        },
        ..input("long generated response with existing flexible note", 0.93)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let report = store.index_report(8);

    assert!(
        record
            .gist_records
            .iter()
            .any(|gist| gist.title == "Generated response clean gist"
                && gist.summary.contains("test-gate"))
    );
    assert_eq!(record.process_reward.notes, vec![existing_note]);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.noisy_record_count, 0);
}

#[test]
fn duplicate_long_generated_response_keeps_clean_gist_on_admission() {
    let mut store = ExperienceStore::new();
    let generated_lesson = "revise_response: 保留远程模型池 test-gate 回滚、重试和上下文压缩证据；当小模型返回上下文溢出时，先压缩 route evidence，再重新选择 worker，避免自动进化循环写入不可检索的长原文。 [reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]"
        .to_owned();
    let first_id = store.record(ExperienceInput {
        prompt: "SmartSteam remote helper smoke context one ".repeat(360),
        lesson: generated_lesson.clone(),
        quality: 0.93,
        ..input("first long generated response", 0.93)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "SmartSteam remote helper smoke context two ".repeat(360),
        lesson: generated_lesson,
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.95,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("duplicate long generated response", 0.96)
    });
    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert!(second.lesson.starts_with("duplicate_reference:"));
    assert!(
        second
            .lesson
            .contains(&format!("canonical_experience_id={first_id}"))
    );
    assert!(
        second
            .gist_records
            .iter()
            .any(|gist| gist.title == "Generated response clean gist"
                && gist.summary.contains("test-gate")),
        "{:?}",
        second.gist_records
    );
    assert!(second.process_reward.notes.iter().any(|note| {
        note.starts_with("experience_index:duplicate_reference:")
            && note.contains(&format!("canonical_id={first_id}"))
    }));
    assert!(
        second
            .process_reward
            .notes
            .iter()
            .any(|note| { note.starts_with("experience_index:generated_response_clean_gist") })
    );
    assert_eq!(report.overlong_record_count, 2);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert_eq!(report.risk_level, "clean");
}

#[test]
fn runtime_backend_error_admission_adds_clean_gist_for_index() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: "SmartSteam model pool route evidence ".repeat(420),
        lesson: "revise_response: Runtime backend error: mistralrs HTTP runtime returned status 400: request exceeds the available context size"
            .to_owned(),
        quality: 0.93,
        ..input("runtime backend context error", 0.93)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let report = store.index_report(8);

    assert!(
        record
            .gist_records
            .iter()
            .any(|gist| gist.summary.contains("compact report and pool context")),
        "{:?}",
        record.gist_records
    );
    assert!(record.quality <= 0.62, "{}", record.quality);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.quality_score >= 0.92, "{report:?}");
    assert!(
        record
            .process_reward
            .notes
            .iter()
            .any(|note| { note.starts_with("experience_index:runtime_backend_error_clean_gist") })
    );
}

#[test]
fn flexible_runtime_backend_error_clean_gist_note_stays_idempotent_on_admission() {
    let mut store = ExperienceStore::new();
    let existing_note = "Experience_Index ： Runtime_Backend_Error_Clean_Gist".to_owned();
    let id = store.record(ExperienceInput {
        prompt: "SmartSteam model pool route evidence ".repeat(420),
        lesson: "revise_response: Runtime backend error: mistralrs HTTP runtime returned status 400: request exceeds the available context size"
            .to_owned(),
        quality: 0.93,
        process_reward: ProcessRewardReport {
            total: 0.93,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![existing_note.clone()],
        },
        ..input("runtime backend context error with existing flexible note", 0.93)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();
    let report = store.index_report(8);

    assert!(
        record
            .gist_records
            .iter()
            .any(|gist| gist.summary.contains("compact report and pool context"))
    );
    assert!(record.quality <= 0.62, "{}", record.quality);
    assert_eq!(record.process_reward.action, RewardAction::Hold);
    assert_eq!(record.process_reward.notes, vec![existing_note]);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 0);
    assert_eq!(report.noisy_record_count, 0);
}

#[test]
fn repair_adds_clean_gist_to_legacy_runtime_backend_error_record() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));
    let mut dirty = store.records()[0].clone();
    dirty.id = store.next_id;
    store.next_id += 1;
    dirty.prompt = "SmartSteam model pool route evidence ".repeat(420);
    dirty.lesson = "revise_response: Runtime backend error: mistralrs HTTP runtime returned status 400: request exceeds the available context size"
        .to_owned();
    dirty.gist_records.clear();
    dirty.quality = 0.93;
    store.records.push(dirty);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == 2)
        .unwrap();

    assert_eq!(before.overlong_without_clean_gist_count, 1);
    assert!(plan.listed_repairs.iter().any(|repair| {
        repair.experience_id == 2 && repair.action == ExperienceRepairAction::AddCleanGist
    }));
    assert!(
        repaired_record
            .gist_records
            .iter()
            .any(|gist| gist.summary.contains("compact report and pool context")),
        "{:?}",
        repaired_record.gist_records
    );
    assert_eq!(after.overlong_without_clean_gist_count, 0);
    assert_eq!(after.noisy_record_count, 0);
    assert!(after.quality_score >= 0.92, "{after:?}");
}

#[test]
fn repair_adds_clean_gist_to_legacy_generated_response_record() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));
    let mut dirty = store.records()[0].clone();
    dirty.id = store.next_id;
    store.next_id += 1;
    dirty.prompt = "SmartSteam remote helper smoke context ".repeat(360);
    dirty.lesson = "Revise_Response: 在 summary 角色中增加关键流程步骤摘要，并把 test-gate 的回滚和重试证据交给 router 复核，避免自动进化循环写入不可检索的长上下文。 [Reflection rejected q=0.451 issues=0 critical=0 severity=info actions=]"
        .to_owned();
    dirty.gist_records.clear();
    dirty.quality = 0.93;
    dirty.process_reward.total = 0.94;
    let dirty_id = dirty.id;
    store.records.push(dirty);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == dirty_id)
        .unwrap();

    assert_eq!(before.overlong_without_clean_gist_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert!(plan.listed_repairs.iter().any(|repair| {
        repair.experience_id == dirty_id
            && repair.action == ExperienceRepairAction::AddCleanGist
            && repair.source == "generated_response_without_clean_gist"
    }));
    assert!(
        repaired_record.gist_records.iter().any(|gist| gist.title
            == "Generated response clean gist"
            && gist.summary.contains("test-gate")
            && !gist.summary.contains("[Reflection")
            && !gist.summary.contains("[reflection")),
        "{:?}",
        repaired_record.gist_records
    );
    assert!(
        repaired_record.quality > 0.90,
        "{}",
        repaired_record.quality
    );
    assert_eq!(after.overlong_without_clean_gist_count, 0);
    assert_eq!(after.noisy_record_count, 0);
    assert!(after.quality_score >= 0.92, "{after:?}");
}

#[test]
fn repair_targets_overlong_runtime_error_without_unwrapping_duplicate_references() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));

    let mut duplicate_reference = store.records()[0].clone();
    duplicate_reference.id = store.next_id;
    store.next_id += 1;
    duplicate_reference.lesson =
        "duplicate_reference: canonical_experience_id=1; original_lesson_chars=240; preview=reuse_response: OK Reflection repair: ground the answer in `Conversation transcript: user: only reply OK assistant:`"
            .to_owned();
    duplicate_reference.gist_records.clear();
    let duplicate_reference_id = duplicate_reference.id;
    store.records.push(duplicate_reference);

    let mut short_runtime_error = store.records()[0].clone();
    short_runtime_error.id = store.next_id;
    store.next_id += 1;
    short_runtime_error.prompt = "short runtime diagnostic".to_owned();
    short_runtime_error.lesson =
        "revise_response: Runtime backend error: failed to connect mistralrs HTTP runtime"
            .to_owned();
    short_runtime_error.gist_records.clear();
    let short_runtime_error_id = short_runtime_error.id;
    store.records.push(short_runtime_error);

    let mut overlong_runtime_error = store.records()[0].clone();
    overlong_runtime_error.id = store.next_id;
    store.next_id += 1;
    overlong_runtime_error.prompt = "SmartSteam model pool route evidence ".repeat(420);
    overlong_runtime_error.lesson =
        "revise_response: Runtime backend error: mistralrs HTTP runtime returned status 400: request exceeds the available context size"
            .to_owned();
    overlong_runtime_error.gist_records.clear();
    let overlong_runtime_error_id = overlong_runtime_error.id;
    store.records.push(overlong_runtime_error);

    let before = store.index_report(16);
    let (repaired, plan) = store.repaired_legacy_metadata_store(16);
    let after = repaired.index_report(16);
    let repaired_duplicate_reference = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_reference_id)
        .unwrap();
    let repaired_short_runtime_error = repaired
        .records()
        .iter()
        .find(|record| record.id == short_runtime_error_id)
        .unwrap();

    assert_eq!(before.overlong_without_clean_gist_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert!(plan.listed_repairs.iter().any(|repair| {
        repair.experience_id == overlong_runtime_error_id
            && repair.action == ExperienceRepairAction::AddCleanGist
    }));
    assert!(
        !plan
            .listed_repairs
            .iter()
            .any(|repair| repair.experience_id == duplicate_reference_id)
    );
    assert!(
        !plan
            .listed_repairs
            .iter()
            .any(|repair| repair.experience_id == short_runtime_error_id)
    );
    assert!(
        repaired_duplicate_reference
            .lesson
            .starts_with("duplicate_reference:")
    );
    assert!(
        repaired_short_runtime_error
            .lesson
            .starts_with("revise_response: Runtime backend error")
    );
    assert_eq!(after.overlong_without_clean_gist_count, 0);
    assert_eq!(after.duplicate_output_count, 0);
    assert_eq!(after.noisy_record_count, 0);
    assert!(after.quality_score >= 0.92, "{after:?}");
}

#[test]
fn full_width_duplicate_reference_is_kept_as_structured_index_record() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));

    let mut duplicate_reference = store.records()[0].clone();
    duplicate_reference.id = store.next_id;
    store.next_id += 1;
    duplicate_reference.lesson =
        "ｄｕｐｌｉｃａｔｅ＿ｒｅｆｅｒｅｎｃｅ： ｃａｎｏｎｉｃａｌ＿ｅｘｐｅｒｉｅｎｃｅ＿ｉｄ＝１； ｏｒｉｇｉｎａｌ＿ｌｅｓｓｏｎ＿ｃｈａｒｓ＝２４０； ｐｒｅｖｉｅｗ＝reuse_response: OK Reflection repair： ground the answer in `Conversation transcript： user： only reply OK assistant：`"
            .to_owned();
    duplicate_reference.gist_records.clear();
    let duplicate_reference_id = duplicate_reference.id;
    let duplicate_reference_lesson = duplicate_reference.lesson.clone();
    store.records.push(duplicate_reference);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let repaired_duplicate_reference = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_reference_id)
        .unwrap();

    assert_eq!(before.duplicate_output_count, 0);
    assert_eq!(before.noisy_record_count, 0);
    assert!(
        !plan
            .listed_repairs
            .iter()
            .any(|repair| repair.experience_id == duplicate_reference_id)
    );
    assert_eq!(
        repaired_duplicate_reference.lesson,
        duplicate_reference_lesson
    );
    assert_eq!(after.duplicate_output_count, 0);
    assert_eq!(after.noisy_record_count, 0);
}

#[test]
fn mixed_case_duplicate_reference_is_kept_as_structured_index_record() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));

    let mut duplicate_reference = store.records()[0].clone();
    duplicate_reference.id = store.next_id;
    store.next_id += 1;
    duplicate_reference.lesson =
        "Duplicate_Reference: Canonical_Experience_Id = 1; Original_Lesson_Chars=240; Preview=reuse_response: OK"
            .to_owned();
    duplicate_reference.gist_records.clear();
    let duplicate_reference_id = duplicate_reference.id;
    let duplicate_reference_lesson = duplicate_reference.lesson.clone();
    store.records.push(duplicate_reference);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let repaired_duplicate_reference = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_reference_id)
        .unwrap();

    assert_eq!(before.duplicate_output_count, 0);
    assert_eq!(before.noisy_record_count, 0);
    assert!(
        !plan
            .listed_repairs
            .iter()
            .any(|repair| repair.experience_id == duplicate_reference_id)
    );
    assert_eq!(
        repaired_duplicate_reference.lesson,
        duplicate_reference_lesson
    );
    assert_eq!(after.duplicate_output_count, 0);
    assert_eq!(after.noisy_record_count, 0);
}

#[test]
fn spaced_duplicate_reference_lesson_is_kept_as_structured_index_record() {
    let mut store = ExperienceStore::new();
    store.record(input("seed stable lesson", 0.91));

    let mut duplicate_reference = store.records()[0].clone();
    duplicate_reference.id = store.next_id;
    store.next_id += 1;
    duplicate_reference.lesson =
        "Duplicate_Reference ： Canonical_Experience_Id ＝ 1； Original_Lesson_Chars=240； Preview=reuse_response: OK"
            .to_owned();
    duplicate_reference.gist_records.clear();
    let duplicate_reference_id = duplicate_reference.id;
    let duplicate_reference_lesson = duplicate_reference.lesson.clone();
    store.records.push(duplicate_reference);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let repaired_duplicate_reference = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_reference_id)
        .unwrap();

    assert_eq!(before.duplicate_output_count, 0);
    assert_eq!(before.noisy_record_count, 0);
    assert!(
        !plan
            .listed_repairs
            .iter()
            .any(|repair| repair.experience_id == duplicate_reference_id)
    );
    assert_eq!(
        repaired_duplicate_reference.lesson,
        duplicate_reference_lesson
    );
    assert_eq!(after.duplicate_output_count, 0);
    assert_eq!(after.noisy_record_count, 0);
}

#[test]
fn index_report_flags_legacy_duplicate_outputs() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route review one".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.91,
        ..input("first duplicate source", 0.91)
    });
    let mut legacy_duplicate = store.records()[0].clone();
    legacy_duplicate.id = store.next_id;
    store.next_id += 1;
    legacy_duplicate.prompt = "Gemma route review two".to_owned();
    legacy_duplicate.lesson = duplicate_lesson;
    legacy_duplicate.quality = 0.92;
    let second_id = legacy_duplicate.id;
    store.records.push(legacy_duplicate);
    store.record(ExperienceInput {
        prompt: "short duplicate should not count".to_owned(),
        lesson: "ok".to_owned(),
        quality: 0.90,
        ..input("short duplicate", 0.90)
    });
    store.record(ExperienceInput {
        prompt: "short duplicate should still not count".to_owned(),
        lesson: "ok".to_owned(),
        quality: 0.90,
        ..input("short duplicate again", 0.90)
    });

    let report = store.index_report(8);

    assert_eq!(report.duplicate_output_count, 1);
    assert_eq!(report.noisy_record_count, 1);
    assert!(report.max_noise_penalty >= 0.12);
    assert!(report.quality_score < 0.75, "{report:?}");
    assert!(report.retrieval_ready);
    assert_eq!(report.risk_level, "degraded");
    assert_eq!(report.recommended_action, "deduplicate_repeated_lessons");
    assert!(report.findings.iter().any(|finding| {
        finding.experience_id == second_id
            && finding.duplicate_of == Some(first_id)
            && finding.reason == "duplicate_output"
    }));
}

#[test]
fn mixed_case_duplicate_reference_note_skips_duplicate_output_detection() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route review one".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.91,
        ..input("first duplicate source", 0.91)
    });
    let mut referenced_duplicate = store.records()[0].clone();
    referenced_duplicate.id = store.next_id;
    store.next_id += 1;
    referenced_duplicate.prompt = "Gemma route review referenced duplicate".to_owned();
    referenced_duplicate.lesson = duplicate_lesson;
    referenced_duplicate.process_reward.notes = vec![format!(
        "Experience_Index:Duplicate_Reference:Canonical_Id = {first_id}:Original_Lesson_Chars=220"
    )];
    store.records.push(referenced_duplicate);

    let report = store.index_report(8);

    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
}

#[test]
fn spaced_full_width_duplicate_reference_note_skips_duplicate_output_detection() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route review one".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.91,
        ..input("first duplicate source", 0.91)
    });
    let mut referenced_duplicate = store.records()[0].clone();
    referenced_duplicate.id = store.next_id;
    store.next_id += 1;
    referenced_duplicate.prompt = "Gemma route review referenced duplicate".to_owned();
    referenced_duplicate.lesson = duplicate_lesson;
    referenced_duplicate.process_reward.notes = vec![format!(
        "Experience_Index ： Duplicate_Reference ： Canonical_Experience_Id ＝ {first_id}； Original_Lesson_Chars ＝ 220"
    )];
    store.records.push(referenced_duplicate);

    let report = store.index_report(8);

    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.findings.is_empty());
}

#[test]
fn admission_duplicate_guard_preserves_existing_flexible_duplicate_reference_note() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route review one".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.91,
        ..input("first duplicate source", 0.91)
    });
    let existing_note = format!(
        "Experience_Index ： Duplicate_Reference ： Canonical_Id ＝ {first_id}； Original_Lesson_Chars ＝ 220"
    );
    let second_id = store.record(ExperienceInput {
        prompt: "Gemma route review referenced duplicate".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.95,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![existing_note.clone()],
        },
        ..input("referenced duplicate source", 0.96)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert_eq!(second.lesson, duplicate_lesson);
    assert_eq!(second.process_reward.notes, vec![existing_note]);
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert!(report.findings.is_empty());
}

#[test]
fn admission_duplicate_guard_rewrites_repeated_long_lessons() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route review one".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.91,
        ..input("first duplicate source", 0.91)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "Gemma route review two".to_owned(),
        lesson: duplicate_lesson,
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("second duplicate source", 0.96)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert!(second.lesson.starts_with("duplicate_reference:"));
    assert!(
        second
            .lesson
            .contains(&format!("canonical_experience_id={first_id}"))
    );
    assert!(second.lesson.contains("original_lesson_chars="));
    assert!((second.quality - 0.72).abs() < 0.0001);
    assert!((second.process_reward.total - 0.72).abs() < 0.0001);
    assert!(second.process_reward.notes.iter().any(|note| {
        note.starts_with("experience_index:duplicate_reference:")
            && note.contains(&format!("canonical_id={first_id}"))
    }));
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert_eq!(report.risk_level, "clean");
    assert_eq!(report.recommended_action, "ready_for_retrieval");
    assert!(report.findings.is_empty());
}

#[test]
fn admission_duplicate_guard_normalizes_punctuation_and_case() {
    let mut store = ExperienceStore::new();
    let canonical_lesson =
        "when Gemma worker is busy keep prompt gated summarize route evidence before sending another request ".repeat(2);
    let punctuated_lesson =
        "When Gemma worker is busy, keep prompt gated; summarize route evidence before sending another request. ".repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence canonical".to_owned(),
        lesson: canonical_lesson,
        quality: 0.91,
        ..input("canonical punctuation duplicate", 0.91)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence punctuation duplicate".to_owned(),
        lesson: punctuated_lesson,
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("punctuation duplicate", 0.96)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert!(second.lesson.starts_with("duplicate_reference:"));
    assert!(
        second
            .lesson
            .contains(&format!("canonical_experience_id={first_id}"))
    );
    assert!(second.quality <= 0.72);
    assert!(second.process_reward.notes.iter().any(|note| {
        note.starts_with("experience_index:duplicate_reference:")
            && note.contains(&format!("canonical_id={first_id}"))
    }));
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
    assert_eq!(report.recommended_action, "ready_for_retrieval");
}

#[test]
fn admission_duplicate_guard_normalizes_cjk_punctuation() {
    let mut store = ExperienceStore::new();
    let canonical_lesson = "当 Gemma worker 忙碌时保持提示门控摘要路由证据再发送请求 ".repeat(4);
    let punctuated_lesson =
        "当 Gemma worker 忙碌时，保持提示门控；摘要路由证据，再发送请求。".repeat(4);
    let first_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence canonical Chinese".to_owned(),
        lesson: canonical_lesson,
        quality: 0.91,
        ..input("canonical cjk punctuation duplicate", 0.91)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence cjk punctuation duplicate".to_owned(),
        lesson: punctuated_lesson,
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("cjk punctuation duplicate", 0.96)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert!(second.lesson.starts_with("duplicate_reference:"));
    assert!(
        second
            .lesson
            .contains(&format!("canonical_experience_id={first_id}"))
    );
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
}

#[test]
fn admission_duplicate_guard_normalizes_full_width_ascii() {
    let mut store = ExperienceStore::new();
    let canonical_lesson =
        "when Rust worker is busy keep prompt gated summarize route evidence before retrying "
            .repeat(2);
    let full_width_lesson =
        "ｗｈｅｎ Ｒｕｓｔ ｗｏｒｋｅｒ ｉｓ ｂｕｓｙ， ｋｅｅｐ ｐｒｏｍｐｔ ｇａｔｅｄ； ｓｕｍｍａｒｉｚｅ ｒｏｕｔｅ ｅｖｉｄｅｎｃｅ ｂｅｆｏｒｅ ｒｅｔｒｙｉｎｇ "
            .repeat(2);
    let first_id = store.record(ExperienceInput {
        prompt: "Rust route evidence canonical full width".to_owned(),
        lesson: canonical_lesson,
        quality: 0.91,
        ..input("canonical full width duplicate", 0.91)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "Rust route evidence full width duplicate".to_owned(),
        lesson: full_width_lesson,
        quality: 0.96,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("full width duplicate", 0.96)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert!(second.lesson.starts_with("duplicate_reference:"));
    assert!(
        second
            .lesson
            .contains(&format!("canonical_experience_id={first_id}"))
    );
    assert!(second.quality <= 0.72);
    assert_eq!(report.duplicate_output_count, 0);
    assert_eq!(report.noisy_record_count, 0);
}

#[test]
fn index_sketch_omits_cjk_punctuation() {
    let mut store = ExperienceStore::new();
    let id = store.record(ExperienceInput {
        prompt: "保持提示门控，摘要路由证据。".to_owned(),
        lesson: "当 Gemma 忙碌时，先压缩上下文；再重试。".to_owned(),
        quality: 0.91,
        ..input("cjk index sketch", 0.91)
    });
    let record = store
        .records()
        .iter()
        .find(|record| record.id == id)
        .unwrap();

    let document = super::index::record_index_document(record);
    let sketch = document
        .text
        .lines()
        .find_map(|line| line.strip_prefix("index_sketch:"))
        .unwrap();

    assert!(sketch.contains("保持提示门控"));
    assert!(!sketch.contains('，'));
    assert!(!sketch.contains('。'));
    assert!(!sketch.contains('；'));
}

#[test]
fn admission_duplicate_guard_leaves_short_repeated_lessons_unchanged() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "short duplicate one".to_owned(),
        lesson: "ok".to_owned(),
        quality: 0.90,
        ..input("short duplicate", 0.90)
    });
    let second_id = store.record(ExperienceInput {
        prompt: "short duplicate two".to_owned(),
        lesson: "ok".to_owned(),
        quality: 0.91,
        ..input("short duplicate again", 0.91)
    });

    let second = store
        .records()
        .iter()
        .find(|record| record.id == second_id)
        .unwrap();
    let report = store.index_report(8);

    assert_eq!(second.lesson, "ok");
    assert!(
        second
            .process_reward
            .notes
            .iter()
            .all(|note| !note.starts_with("experience_index:duplicate_reference:"))
    );
    assert_eq!(report.duplicate_output_count, 0);
}

#[test]
fn retrieval_prefers_canonical_lesson_over_duplicate_reference() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy, keep the prompt gated and summarize the route evidence before sending another request ".repeat(2);
    let canonical_id = store.record(ExperienceInput {
        prompt: "Gemma worker busy route evidence".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.94,
        process_reward: ProcessRewardReport {
            total: 0.95,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("canonical route evidence", 0.94)
    });
    let duplicate_id = store.record(ExperienceInput {
        prompt: "Gemma worker duplicate route evidence".to_owned(),
        lesson: duplicate_lesson,
        quality: 0.99,
        process_reward: ProcessRewardReport {
            total: 0.99,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("duplicate route evidence", 0.99)
    });

    let matches =
        store.retrieve_lessons("Gemma worker busy route evidence", TaskProfile::Coding, 2);

    assert!(!matches.is_empty());
    assert_eq!(matches[0].id, canonical_id, "{matches:?}");
    assert_ne!(matches[0].id, duplicate_id, "{matches:?}");
}

#[test]
fn retrieval_penalizes_unstructured_long_transcript_index_noise() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: format!(
            "Conversation transcript:\nuser: rust router\nassistant: {}\nuser: unrelated notes\n{}",
            "adaptive router ".repeat(500),
            "random project chatter ".repeat(800)
        ),
        lesson: "a very long transcript should not outrank a concise indexed router lesson"
            .repeat(120),
        quality: 0.99,
        ..input("long noisy", 0.99)
    });
    store.record(ExperienceInput {
        prompt: "Rust adaptive router feedback".to_owned(),
        lesson: "prefer token-window feedback for router stability".to_owned(),
        quality: 0.82,
        ..input("clean router", 0.82)
    });

    let matches = store.retrieve_lessons("Rust router feedback", TaskProfile::Coding, 2);
    let index_report = store.index_report(8);

    assert!(!matches.is_empty());
    assert!(index_report.quality_score < 0.75, "{index_report:?}");
    assert!(!index_report.retrieval_ready, "{index_report:?}");
    assert_eq!(index_report.risk_level, "blocked");
    assert_eq!(
        index_report.recommended_action,
        "pause_chat_and_add_clean_gists"
    );
    assert_eq!(index_report.overlong_record_count, 1);
    assert_eq!(index_report.overlong_without_clean_gist_count, 1);
    assert!(index_report.max_record_chars > 2_400, "{index_report:?}");
    assert!(index_report.max_noise_penalty >= 0.30, "{index_report:?}");
    assert!(index_report.findings.iter().any(|finding| {
        finding
            .reason
            .contains("unstructured_long_transcript+transcript_prompt_without_clean_lesson")
    }));
    assert!(
        matches[0].lesson.contains("prefer token-window feedback"),
        "{matches:?}"
    );
}

#[test]
fn retrieval_demotes_transcript_metadata_lesson_noise() {
    let mut store = ExperienceStore::new();
    let noisy_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: use for i in 0..10\nuser: 你好\nassistant: 你好，我在".to_owned(),
        lesson: "accepted_pattern quality=0.991 overlap=0.982 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.99,
        process_reward: ProcessRewardReport {
            total: 0.96,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("noisy metadata", 0.99)
    });
    let clean_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10 and println".to_owned(),
        quality: 0.82,
        ..input("clean rust loop", 0.82)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert!(report.has_matches());
    assert!(report.retrieval_noise_penalized_candidates >= 1);
    assert!(report.suppressed_prompt_index_candidates >= 1);
    assert!(report.max_retrieval_noise_penalty > 0.0);
    assert_eq!(report.matches[0].id, clean_id, "{report:?}");
    assert!(
        report
            .matches
            .iter()
            .all(|item| item.id != noisy_id || item.score < report.matches[0].score),
        "{report:?}"
    );
}

#[test]
fn index_report_surfaces_retrieval_metadata_lesson_noise() {
    let mut store = ExperienceStore::new();
    let noisy_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: use for i in 0..10"
            .to_owned(),
        lesson: "accepted_pattern quality=0.991 overlap=0.982 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.99,
        ..input("metadata noise", 0.99)
    });
    store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a reusable Rust range loop with for i in 0..10 and println".to_owned(),
        quality: 0.82,
        ..input("clean rust loop", 0.82)
    });

    let report = store.index_report(8);

    assert_eq!(report.noisy_record_count, 1);
    assert_eq!(report.duplicate_output_count, 0);
    assert!(report.max_noise_penalty >= 0.28, "{report:?}");
    assert!(report.quality_score < 0.75, "{report:?}");
    assert!(report.retrieval_ready, "{report:?}");
    assert_eq!(report.risk_level, "degraded");
    assert_eq!(report.recommended_action, "review_index_findings");
    assert!(report.findings.iter().any(|finding| {
        finding.experience_id == noisy_id
            && finding.reason == "legacy_metadata_lesson_missing_clean_gist"
    }));
}

#[test]
fn retrieval_report_counts_noise_filtered_candidates() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 你好\nassistant: 你好，我在".to_owned(),
        lesson: "accepted_pattern quality=0.100 overlap=0.010 issues=1 max_severity=critical"
            .to_owned(),
        quality: 0.10,
        ..input("filtered metadata", 0.10)
    });
    store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10 and println".to_owned(),
        quality: 0.82,
        ..input("clean rust loop", 0.82)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert!(report.has_matches());
    assert_eq!(report.match_count(), 1);
    assert_eq!(report.retrieval_noise_penalized_candidates, 1);
    assert_eq!(report.retrieval_noise_filtered_candidates, 1);
    assert_eq!(report.suppressed_prompt_index_candidates, 1);
    assert!(report.max_retrieval_noise_penalty >= 0.44);
}

#[test]
fn retrieval_demotes_role_labeled_lesson_residue() {
    let mut store = ExperienceStore::new();
    let noisy_id = store.record(ExperienceInput {
        prompt: "Rust router debugging transcript residue".to_owned(),
        lesson:
            "AsThought: AsThought: assistant: prefer token-window feedback for router stability"
                .to_owned(),
        quality: 0.99,
        process_reward: ProcessRewardReport {
            total: 0.96,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("role labeled residue", 0.99)
    });
    let clean_id = store.record(ExperienceInput {
        prompt: "Rust adaptive router feedback".to_owned(),
        lesson: "prefer token-window feedback for router stability".to_owned(),
        quality: 0.82,
        ..input("clean router", 0.82)
    });

    let report = store.retrieval_report("Rust router feedback", TaskProfile::Coding, 5);
    let index_report = store.index_report(8);

    assert!(report.has_matches());
    assert_eq!(report.retrieval_noise_penalized_candidates, 1);
    assert_eq!(report.max_retrieval_noise_penalty, 0.44);
    assert_eq!(report.matches[0].id, clean_id, "{report:?}");
    assert!(
        report
            .matches
            .iter()
            .all(|item| item.id != noisy_id || item.score < report.matches[0].score),
        "{report:?}"
    );
    assert!(index_report.findings.iter().any(|finding| {
        finding.experience_id == noisy_id && finding.reason == "transcript_lesson"
    }));
}

#[test]
fn render_experience_hint_uses_gist_instead_of_metadata_lesson() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:Conversation transcript importance=0.850 tokens=42 summary=这是一个 Rust for 循环代码示例，使用 for i in 0..10 并 println 输出"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Rust for 循环代码示例"));
    assert!(!hint.contains("accepted_pattern"));
    assert!(!hint.contains("Conversation transcript"));
}

#[test]
fn render_experience_hint_uses_last_summary_field_in_gist_hint() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:title with summary=literal metadata importance=0.850 tokens=42 summary=Use compact route evidence for slow Gemma checks"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Use compact route evidence for slow Gemma checks"));
    assert!(!hint.contains("literal metadata importance"));
}

#[test]
fn render_experience_hint_accepts_summary_field_at_start_of_gist_hint() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "summary=Use compact route evidence when imported gist hints omit a title".to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Use compact route evidence when imported gist hints omit a title"));
    assert!(!hint.contains("prior accepted result has no reusable lesson text"));
}

#[test]
fn render_experience_hint_ignores_summary_substrings_without_field_boundary() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:nosummary=stale metadata summary=Use compact route evidence from a real summary field"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Use compact route evidence from a real summary field"));
    assert!(!hint.contains("stale metadata"));
}

#[test]
fn render_experience_hint_strips_generated_and_response_prefixes_from_gist_summary() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:title importance=0.850 tokens=42 summary=Revise_Response: assistant: AsThought: AsThought: Use compact route evidence [Reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Use compact route evidence"));
    assert!(!hint.contains("Revise_Response:"));
    assert!(!hint.contains("revise_response:"));
    assert!(!hint.contains("[Reflection"));
    assert!(!hint.contains("[reflection"));
    assert!(!hint.contains("AsThought"));
    assert!(!hint.contains("assistant:"));
}

#[test]
fn render_experience_hint_accepts_full_width_summary_field_in_gist_hint() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:title summary=literal metadata importance=0.850 tokens=42 ｓｕｍｍａｒｙ＝Ｒｅｕｓｅ＿Ｒｅｓｐｏｎｓｅ： ａｓｓｉｓｔａｎｔ： ＡｓＴｈｏｕｇｈｔ： Use compact route evidence [Reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.contains("Use compact route evidence"));
    assert!(!hint.contains("literal metadata importance"));
    assert!(!hint.contains("summary＝"));
    assert!(!hint.contains("ｓｕｍｍａｒｙ＝"));
    assert!(!hint.contains("Reuse_Response："));
    assert!(!hint.contains("Ｒｅｕｓｅ＿Ｒｅｓｐｏｎｓｅ："));
    assert!(!hint.contains("assistant："));
    assert!(!hint.contains("ａｓｓｉｓｔａｎｔ："));
    assert!(!hint.contains("AsThought"));
    assert!(!hint.contains("ＡｓＴｈｏｕｇｈｔ"));
}

#[test]
fn render_experience_hint_strips_generated_prefix_from_direct_lesson() {
    let hint = render_experience_hint(&ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "Reuse_Response: assistant: AsThought: AsThought: Use compact route evidence before retrying slow Gemma checks [Reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: Vec::new(),
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: RewardAction::Reinforce,
        used_memory_count: 0,
        stored_runtime_kv_memory_ids: Vec::new(),
        route_threshold: 0.0,
        route_attention_tokens: 0,
        route_fast_tokens: 0,
        route_attention_fraction: 0.0,
        runtime_model_id: None,
        runtime_selected_adapter: None,
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    });

    assert!(hint.starts_with("Use compact route evidence"));
    assert!(!hint.contains("Reuse_Response:"));
    assert!(!hint.contains("reuse_response:"));
    assert!(!hint.contains("[Reflection"));
    assert!(!hint.contains("[reflection"));
    assert!(!hint.contains("AsThought"));
    assert!(!hint.contains("assistant:"));
}

#[test]
fn retrieval_prefers_transcript_with_matching_first_user_anchor() {
    let mut store = ExperienceStore::new();
    let greeting_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 你好\nassistant: 你好，请问有什么可以帮你\nuser: 帮我用rust输出一段for循环代码\nassistant: for i in 0..10"
            .to_owned(),
        lesson: "accepted_pattern quality=0.780 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.88,
        gist_records: vec![gist(
            "这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出",
            GistLevel::Document,
            0.86,
        )],
        process_reward: ProcessRewardReport {
            total: 0.84,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("greeting anchored metadata", 0.88)
    });
    let direct_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        gist_records: vec![gist(
            "这是一个简单的 Rust for 循环示例，使用 for i in 0..10 并 println 输出",
            GistLevel::Document,
            0.85,
        )],
        process_reward: ProcessRewardReport {
            total: 0.78,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("direct anchored metadata", 0.78)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert!(report.has_matches());
    assert_eq!(report.matches[0].id, direct_id, "{report:?}");
    assert!(
        report
            .matches
            .iter()
            .all(|item| item.id != greeting_id || item.score < report.matches[0].score),
        "{report:?}"
    );
}

#[test]
fn retrieval_demotes_high_quality_records_without_task_anchors() {
    let mut store = ExperienceStore::new();
    let control_id = store.record(ExperienceInput {
        prompt: "benchmark auto replay control plane".to_owned(),
        lesson: "benchmark_auto_replay_seed:v2:control_plane reinforce router threshold".to_owned(),
        quality: 1.0,
        process_reward: ProcessRewardReport {
            total: 1.0,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("control seed", 1.0)
    });
    let rust_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show Rust code for a for loop using for i in 0..10 and println output".to_owned(),
        quality: 0.72,
        ..input("rust loop anchor", 0.72)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert!(report.has_matches());
    assert_eq!(report.matches[0].id, rust_id, "{report:?}");
    assert!(
        report
            .matches
            .iter()
            .all(|item| item.id != control_id || item.score < report.matches[0].score),
        "{report:?}"
    );
}

#[test]
fn retrieval_demotes_metadata_gist_when_original_prompt_mismatches_task() {
    let mut store = ExperienceStore::new();
    let cli_id = store.record(ExperienceInput {
        prompt: "用中文说一句当前 CLI 是否可用".to_owned(),
        lesson: "accepted_pattern quality=0.926 overlap=1.000 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.93,
        gist_records: vec![gist(
            "Prototype inference result for a Rust trait boundary and model control plane",
            GistLevel::Document,
            0.92,
        )],
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("cli metadata", 0.93)
    });
    let rust_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        gist_records: vec![gist(
            "这是一个简单的 Rust for 循环代码，使用 for i in 0..10 并 println 输出",
            GistLevel::Document,
            0.85,
        )],
        process_reward: ProcessRewardReport {
            total: 0.78,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("rust loop metadata", 0.78)
    });

    let report = store.retrieval_report("帮我用rust输出一段for循环代码", TaskProfile::Coding, 5);

    assert!(report.has_matches());
    assert_eq!(report.matches[0].id, rust_id, "{report:?}");
    assert!(
        report
            .matches
            .iter()
            .all(|item| item.id != cli_id || item.score < report.matches[0].score),
        "{report:?}"
    );
}

#[test]
fn retrieval_filters_metadata_lesson_when_only_metadata_terms_match() {
    let mut store = ExperienceStore::new();
    let metadata_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.50,
        gist_records: vec![gist(
            "CLI status note unrelated to metadata scoring fields",
            GistLevel::Document,
            0.85,
        )],
        process_reward: ProcessRewardReport {
            total: 0.0,
            action: RewardAction::Hold,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("metadata-only match", 0.50)
    });

    let report = store.retrieval_report(
        "accepted_pattern quality overlap max_severity",
        TaskProfile::Coding,
        5,
    );

    assert_eq!(report.total_records, 1);
    assert_eq!(report.suppressed_prompt_index_candidates, 1);
    assert_eq!(report.retrieval_noise_penalized_candidates, 1);
    assert_eq!(report.retrieval_noise_filtered_candidates, 1);
    assert!(
        report.matches.iter().all(|item| item.id != metadata_id),
        "{report:?}"
    );
}

#[test]
fn hygiene_report_surfaces_cross_task_shell_transcript_pollution() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: clean loop\nuser: Bash command\nssh -o ConnectTimeout=8 host 'echo ok'\nassistant: gitlab.local merge_requests"
            .to_owned(),
        lesson: "polluted answer mixed with shell and merge_requests context".to_owned(),
        quality: 0.97,
        ..input("polluted", 0.97)
    });
    store.record(ExperienceInput {
        prompt: "Rust loops".to_owned(),
        lesson: "for i in 0..10 stays clean".to_owned(),
        quality: 0.88,
        ..input("clean", 0.88)
    });

    let report = store.hygiene_report(8);

    assert_eq!(report.total_records, 2);
    assert_eq!(report.finding_count, 1);
    assert_eq!(report.quarantine_candidate_count, 1);
    assert_eq!(report.findings[0].experience_id, polluted_id);
    assert_eq!(
        report.findings[0].severity,
        ExperienceHygieneSeverity::QuarantineCandidate
    );
    assert!(
        report.findings[0]
            .markers
            .contains(&"bash_command".to_owned())
    );
    assert!(
        report.findings[0]
            .markers
            .contains(&"gitlab_local".to_owned())
    );
}

#[test]
fn hygiene_report_limit_zero_keeps_counts_without_listed_findings() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: rust loop\nassistant: clean loop\nuser: Bash command\nssh -o ConnectTimeout=8 host 'echo ok'\nassistant: gitlab.local"
            .to_owned(),
        lesson: "polluted answer mixed with shell context".to_owned(),
        quality: 0.97,
        ..input("polluted", 0.97)
    });

    let report = store.hygiene_report(0);

    assert_eq!(report.total_records, 1);
    assert_eq!(report.finding_count, 1);
    assert_eq!(report.quarantine_candidate_count, 1);
    assert!(report.findings.is_empty());
}

#[test]
fn hygiene_report_surfaces_legacy_metadata_lessons_as_watch_items() {
    let mut store = ExperienceStore::new();
    let with_gist_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![gist(
            "这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出",
            GistLevel::Document,
            0.84,
        )],
        ..input("legacy with gist", 0.78)
    });
    let without_gist_id = store.record(ExperienceInput {
        prompt: "Rust loop answer".to_owned(),
        lesson: "assistant: AsThought: rejected_pattern quality=0.120 issues=1 critical=1 max_severity=critical"
            .to_owned(),
        ..input("legacy without gist", 0.12)
    });

    let report = store.hygiene_report(8);

    assert_eq!(report.total_records, 2);
    assert_eq!(report.finding_count, 2);
    assert_eq!(report.watch_count, 2);
    assert_eq!(report.quarantine_candidate_count, 0);
    assert_eq!(report.legacy_metadata_lesson_count, 2);
    assert_eq!(report.legacy_metadata_without_clean_gist_count, 1);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.experience_id == with_gist_id
                && finding.severity == ExperienceHygieneSeverity::Watch
                && finding.markers.contains(&"clean_gist_fallback".to_owned()))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.experience_id == without_gist_id
                && finding.severity == ExperienceHygieneSeverity::Watch
                && finding.markers.contains(&"missing_clean_gist".to_owned()))
    );
}

#[test]
fn index_report_limit_zero_keeps_counts_without_listed_findings() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "SmartSteam remote helper smoke context ".repeat(360),
        lesson: "generated response without clean gist".to_owned(),
        quality: 0.93,
        ..input("long generated response", 0.93)
    });

    let report = store.index_report(0);

    assert_eq!(report.total_records, 1);
    assert_eq!(report.overlong_record_count, 1);
    assert_eq!(report.overlong_without_clean_gist_count, 1);
    assert_eq!(report.noisy_record_count, 1);
    assert!(report.findings.is_empty());
}

#[test]
fn legacy_metadata_repair_plan_converts_clean_gist_to_reusable_lesson() {
    let mut store = ExperienceStore::new();
    let repairable_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![gist(
            "Reuse_Response: assistant: AsThought: 这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出 [Reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]",
            GistLevel::Document,
            0.84,
        )],
        ..input("legacy repairable", 0.78)
    });
    store.record(ExperienceInput {
        prompt: "Rust loop answer".to_owned(),
        lesson: "rejected_pattern quality=0.120 issues=1 critical=1 max_severity=critical"
            .to_owned(),
        ..input("legacy without gist", 0.12)
    });

    let (repaired, plan) = store.repaired_legacy_metadata_store(8);

    assert_eq!(plan.total_records, 2);
    assert_eq!(plan.legacy_metadata_lesson_count, 2);
    assert_eq!(plan.repairable_legacy_metadata_lesson_count, 1);
    assert_eq!(plan.skipped_missing_clean_gist_count, 1);
    assert_eq!(plan.skipped_quarantine_candidate_count, 0);
    assert_eq!(plan.listed_skipped_missing_clean_gist.len(), 1);
    assert_eq!(
        plan.listed_skipped_missing_clean_gist[0].reason,
        "missing_clean_gist"
    );
    assert_eq!(plan.listed_skipped_missing_clean_gist[0].gist_count, 0);
    assert_eq!(
        plan.remaining_legacy_metadata_lesson_count_after_repair(),
        1
    );
    assert_eq!(plan.remaining_watch_count_after_repair(), 1);
    assert_eq!(plan.remaining_quarantine_candidate_count_after_repair(), 0);
    assert_eq!(plan.projected_after_repair.total_records, 2);
    assert_eq!(plan.projected_after_repair.hygiene_finding_count, 1);
    assert_eq!(plan.projected_after_repair.hygiene_watch_count, 1);
    assert_eq!(
        plan.projected_after_repair
            .hygiene_quarantine_candidate_count,
        0
    );
    assert_eq!(plan.projected_after_repair.legacy_metadata_lesson_count, 1);
    assert_eq!(
        plan.projected_after_repair
            .legacy_metadata_without_clean_gist_count,
        1
    );
    assert_eq!(plan.listed_repairs[0].experience_id, repairable_id);
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::ReuseResponse
    );
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == repairable_id)
        .unwrap();
    assert!(repaired_record.lesson.starts_with("reuse_response:"));
    assert!(repaired_record.lesson.contains("Rust for 循环示例"));
    assert!(!repaired_record.lesson.contains("Reuse_Response:"));
    assert!(!repaired_record.lesson.contains("[Reflection"));
    assert!(!repaired_record.lesson.contains("[reflection"));
    assert!(!repaired_record.lesson.contains("AsThought"));
    assert!(!repaired_record.lesson.contains("accepted_pattern"));
    assert!(
        repaired_record
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("experience_repair:legacy_metadata_lesson"))
    );
}

#[test]
fn legacy_metadata_repair_preserves_existing_flexible_repair_note() {
    let mut store = ExperienceStore::new();
    let existing_note =
        "Experience_Repair ： Legacy_Metadata_Lesson ： Source＝Clean_Gist； Action＝Reuse_Response"
            .to_owned();
    let repairable_id = store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![gist(
            "Reuse_Response: assistant: AsThought: 这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出 [Reflection accepted q=0.842 issues=0 critical=0 severity=info actions=]",
            GistLevel::Document,
            0.84,
        )],
        process_reward: ProcessRewardReport {
            total: 0.78,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![existing_note.clone()],
        },
        ..input("legacy repairable with existing repair note", 0.78)
    });

    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == repairable_id)
        .unwrap();

    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::ReuseResponse
    );
    assert!(repaired_record.lesson.starts_with("reuse_response:"));
    assert!(repaired_record.lesson.contains("Rust for 循环示例"));
    assert_eq!(repaired_record.process_reward.notes, vec![existing_note]);
}

#[test]
fn legacy_metadata_repair_plan_limit_zero_keeps_counts_without_listed_samples() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: rust loop\nassistant:".to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![gist(
            "Use for item in items and keep the response focused on Rust iteration.",
            GistLevel::Document,
            0.84,
        )],
        ..input("legacy repairable", 0.78)
    });
    store.record(ExperienceInput {
        prompt: "Rust loop answer".to_owned(),
        lesson: "rejected_pattern quality=0.120 issues=1 critical=1 max_severity=critical"
            .to_owned(),
        ..input("legacy without gist", 0.12)
    });

    let (_repaired, plan) = store.repaired_legacy_metadata_store(0);

    assert_eq!(plan.total_records, 2);
    assert_eq!(plan.legacy_metadata_lesson_count, 2);
    assert_eq!(plan.repairable_legacy_metadata_lesson_count, 1);
    assert_eq!(plan.skipped_missing_clean_gist_count, 1);
    assert_eq!(plan.projected_after_repair.legacy_metadata_lesson_count, 1);
    assert!(plan.listed_repairs.is_empty());
    assert!(plan.listed_skipped_missing_clean_gist.is_empty());
    assert!(plan.listed_skipped_quarantine_candidates.is_empty());
}

#[test]
fn legacy_metadata_repair_uses_full_width_rejected_pattern_action() {
    let mut store = ExperienceStore::new();
    let repairable_id = store.record(ExperienceInput {
        prompt: "Rust router answer".to_owned(),
        lesson: "ＡｓＴｈｏｕｇｈｔ： ａｓｓｉｓｔａｎｔ： ｒｅｊｅｃｔｅｄ＿ｐａｔｔｅｒｎ："
            .to_owned(),
        gist_records: vec![gist(
            "Ｒｅｖｉｓｅ＿Ｒｅｓｐｏｎｓｅ： ａｓｓｉｓｔａｎｔ： ＡｓＴｈｏｕｇｈｔ： Keep route evidence compact before retrying slow model workers [Reflection rejected q=0.451 issues=0 critical=0 severity=info actions=]",
            GistLevel::Document,
            0.84,
        )],
        ..input("full width rejected metadata repair", 0.78)
    });

    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == repairable_id)
        .unwrap();

    assert_eq!(plan.legacy_metadata_lesson_count, 1);
    assert_eq!(plan.repairable_legacy_metadata_lesson_count, 1);
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::ReviseResponse
    );
    assert_eq!(
        repaired_record.lesson,
        "revise_response: Keep route evidence compact before retrying slow model workers"
    );
    assert!(!repaired_record.lesson.contains("rejected_pattern"));
    assert!(!repaired_record.lesson.contains("Revise_Response："));
    assert!(
        !repaired_record
            .lesson
            .contains("Ｒｅｖｉｓｅ＿Ｒｅｓｐｏｎｓｅ：")
    );
    assert!(!repaired_record.lesson.contains("AsThought"));
    assert!(!repaired_record.lesson.contains("ＡｓＴｈｏｕｇｈｔ"));
}

#[test]
fn legacy_metadata_repair_deduplicates_historical_index_outputs() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy keep the prompt gated and summarize route evidence before sending another request ".repeat(2);
    let punctuated_duplicate_lesson =
        "When the Gemma worker is busy, keep the prompt gated; and summarize route evidence before sending another request. ".repeat(2);
    let canonical_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence canonical".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.94,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("canonical historical duplicate", 0.94)
    });
    let mut historical_duplicate = store.records()[0].clone();
    historical_duplicate.id = store.next_id;
    store.next_id += 1;
    historical_duplicate.prompt = "Gemma route evidence historical duplicate".to_owned();
    historical_duplicate.lesson = punctuated_duplicate_lesson;
    historical_duplicate.quality = 0.97;
    historical_duplicate.process_reward.total = 0.97;
    historical_duplicate.process_reward.notes.clear();
    let duplicate_id = historical_duplicate.id;
    store.records.push(historical_duplicate);

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after = repaired.index_report(8);
    let after_duplicate = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_id)
        .unwrap();

    assert_eq!(before.duplicate_output_count, 1);
    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(plan.index_duplicate_output_count, 1);
    assert_eq!(plan.index_noisy_record_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_duplicate_output_count, 0);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert!(plan.projected_after_repair.index_quality_score >= 0.92);
    assert!(plan.projected_after_repair.index_retrieval_ready);
    assert_eq!(plan.projected_after_repair.index_risk_level, "clean");
    assert_eq!(after.duplicate_output_count, 0);
    assert_eq!(after.noisy_record_count, 0);
    assert!(after.retrieval_ready);
    assert_eq!(after.risk_level, "clean");
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::DedupeReference
    );
    assert!(after_duplicate.lesson.starts_with("duplicate_reference:"));
    assert!(
        after_duplicate
            .lesson
            .contains(&format!("canonical_experience_id={canonical_id}"))
    );
    assert!(after_duplicate.quality <= 0.72);
    assert!(
        after_duplicate
            .process_reward
            .notes
            .iter()
            .any(|note| { note == "experience_repair:index_quality:action=dedupe_reference" })
    );
}

#[test]
fn index_repair_preserves_existing_flexible_repair_note() {
    let mut store = ExperienceStore::new();
    let duplicate_lesson =
        "when the Gemma worker is busy keep the prompt gated and summarize route evidence before sending another request ".repeat(2);
    let punctuated_duplicate_lesson =
        "When the Gemma worker is busy, keep the prompt gated; and summarize route evidence before sending another request. ".repeat(2);
    let canonical_id = store.record(ExperienceInput {
        prompt: "Gemma route evidence canonical".to_owned(),
        lesson: duplicate_lesson.clone(),
        quality: 0.94,
        process_reward: ProcessRewardReport {
            total: 0.94,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("canonical historical duplicate", 0.94)
    });
    let existing_note = "Experience_Repair ： Index_Quality ： Action＝Dedupe_Reference".to_owned();
    let mut historical_duplicate = store.records()[0].clone();
    historical_duplicate.id = store.next_id;
    store.next_id += 1;
    historical_duplicate.prompt = "Gemma route evidence historical duplicate".to_owned();
    historical_duplicate.lesson = punctuated_duplicate_lesson;
    historical_duplicate.quality = 0.97;
    historical_duplicate.process_reward.total = 0.97;
    historical_duplicate.process_reward.notes = vec![existing_note.clone()];
    let duplicate_id = historical_duplicate.id;
    store.records.push(historical_duplicate);

    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let after_duplicate = repaired
        .records()
        .iter()
        .find(|record| record.id == duplicate_id)
        .unwrap();

    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::DedupeReference
    );
    assert!(after_duplicate.lesson.starts_with("duplicate_reference:"));
    assert!(
        after_duplicate
            .lesson
            .contains(&format!("canonical_experience_id={canonical_id}"))
    );
    assert_eq!(after_duplicate.process_reward.notes, vec![existing_note]);
}

#[test]
fn legacy_metadata_repair_strips_transcript_context_from_index_lessons() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Rust loop answer".to_owned(),
        lesson: "Reuse_Response: show a small Rust for loop with a range and println output. reflection repair: ground the answer in Conversation transcript:\nuser: old unrelated request\nassistant: stale context"
            .to_owned(),
        quality: 0.90,
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("transcript lesson repair", 0.90)
    });

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(before.duplicate_output_count, 0);
    assert!(before.findings[0].reason.contains("transcript_lesson"));
    assert_eq!(plan.index_noisy_record_count, 1);
    assert_eq!(plan.index_duplicate_output_count, 0);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert_eq!(plan.projected_after_repair.index_duplicate_output_count, 0);
    assert!(plan.projected_after_repair.index_quality_score >= 0.92);
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::StripTranscriptContext
    );
    assert!(repaired_record.lesson.starts_with("reuse_response:"));
    assert!(repaired_record.lesson.contains("small Rust for loop"));
    assert!(!repaired_record.lesson.contains("Reuse_Response:"));
    assert!(!repaired_record.lesson.contains("Reflection repair:"));
    assert!(!repaired_record.lesson.contains("reflection repair:"));
    assert!(!repaired_record.lesson.contains("Conversation transcript:"));
    assert_eq!(repaired_record.process_reward.action, RewardAction::Hold);
    assert!(
        repaired_record.process_reward.notes.iter().any(|note| {
            note == "experience_repair:index_quality:action=strip_transcript_context"
        })
    );
}

#[test]
fn legacy_metadata_repair_strips_role_labeled_lesson_residue() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Rust router answer".to_owned(),
        lesson:
            "AsThought: AsThought: assistant: prefer token-window feedback for router stability"
                .to_owned(),
        quality: 0.90,
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("role labeled lesson repair", 0.90)
    });

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(before.findings[0].reason, "transcript_lesson");
    assert_eq!(plan.index_noisy_record_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert_eq!(plan.projected_after_repair.index_risk_level, "clean");
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::StripTranscriptContext
    );
    assert_eq!(
        repaired_record.lesson,
        "reuse_response: prefer token-window feedback for router stability"
    );
    assert!(!repaired_record.lesson.contains("AsThought"));
    assert!(!repaired_record.lesson.contains("assistant:"));
    assert_eq!(repaired_record.process_reward.action, RewardAction::Hold);
    assert!(
        repaired_record.process_reward.notes.iter().any(|note| {
            note == "experience_repair:index_quality:action=strip_transcript_context"
        })
    );
}

#[test]
fn legacy_metadata_repair_strips_full_width_role_labeled_lesson_residue() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Rust router answer".to_owned(),
        lesson: "ＡｓＴｈｏｕｇｈｔ： ＡｓＴｈｏｕｇｈｔ： ａｓｓｉｓｔａｎｔ： prefer token-window feedback for router stability"
            .to_owned(),
        quality: 0.90,
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("full width role labeled lesson repair", 0.90)
    });

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(before.findings[0].reason, "transcript_lesson");
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert_eq!(
        repaired_record.lesson,
        "reuse_response: prefer token-window feedback for router stability"
    );
    assert!(!repaired_record.lesson.contains("AsThought"));
    assert!(!repaired_record.lesson.contains("ＡｓＴｈｏｕｇｈｔ"));
    assert!(!repaired_record.lesson.contains("assistant："));
    assert!(!repaired_record.lesson.contains("ａｓｓｉｓｔａｎｔ："));
    assert_eq!(repaired_record.process_reward.action, RewardAction::Hold);
}

#[test]
fn legacy_metadata_repair_strips_short_reuse_response_transcript_context() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Short OK answer".to_owned(),
        lesson: "Reuse_Response: OK reflection repair: ground the answer in `Conversation transcript: user: only reply OK assistant:`; address actions `expand_short_answer`"
            .to_owned(),
        quality: 0.88,
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("short transcript lesson repair", 0.88)
    });

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert_eq!(plan.projected_after_repair.index_duplicate_output_count, 0);
    assert!(plan.projected_after_repair.index_quality_score >= 0.92);
    assert_eq!(
        plan.listed_repairs[0].action,
        ExperienceRepairAction::StripTranscriptContext
    );
    assert_eq!(repaired_record.lesson, "reuse_response: OK");
    assert_eq!(repaired_record.process_reward.action, RewardAction::Hold);
}

#[test]
fn legacy_metadata_repair_strips_full_width_reuse_response_transcript_context() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Short OK answer".to_owned(),
        lesson: "Reuse_Response： OK reflection repair： ground the answer in `Conversation transcript： user： only reply OK assistant：`; address actions `expand_short_answer`"
            .to_owned(),
        quality: 0.88,
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("full width short transcript lesson repair", 0.88)
    });

    let before = store.index_report(8);
    let (repaired, plan) = store.repaired_legacy_metadata_store(8);
    let repaired_record = repaired
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert_eq!(before.noisy_record_count, 1);
    assert_eq!(plan.repairable_index_record_count, 1);
    assert_eq!(plan.projected_after_repair.index_noisy_record_count, 0);
    assert_eq!(repaired_record.lesson, "reuse_response: OK");
    assert!(!repaired_record.lesson.contains("Reuse_Response："));
    assert!(!repaired_record.lesson.contains("reflection repair："));
    assert!(!repaired_record.lesson.contains("Conversation transcript："));
    assert_eq!(repaired_record.process_reward.action, RewardAction::Hold);
}

#[test]
fn legacy_metadata_repair_skips_quarantine_candidates() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: rust loop\nassistant: ok\nuser: Bash command\nssh -o ConnectTimeout=8 host 'echo ok'\nassistant: gitlab.local merge_requests"
            .to_owned(),
        lesson: "accepted_pattern quality=0.903 overlap=0.938 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![gist(
            "这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出",
            GistLevel::Document,
            0.84,
        )],
        ..input("polluted legacy", 0.90)
    });

    let (_repaired, plan) = store.repaired_legacy_metadata_store(8);

    assert_eq!(plan.legacy_metadata_lesson_count, 1);
    assert_eq!(plan.repairable_legacy_metadata_lesson_count, 0);
    assert_eq!(plan.skipped_quarantine_candidate_count, 1);
    assert_eq!(plan.listed_skipped_quarantine_candidates.len(), 1);
    assert_eq!(
        plan.listed_skipped_quarantine_candidates[0].reason,
        "quarantine_candidate"
    );
    assert_eq!(plan.listed_skipped_quarantine_candidates[0].gist_count, 1);
    assert_eq!(
        plan.remaining_legacy_metadata_lesson_count_after_repair(),
        1
    );
    assert_eq!(plan.remaining_watch_count_after_repair(), 0);
    assert_eq!(plan.remaining_quarantine_candidate_count_after_repair(), 1);
    assert_eq!(plan.projected_after_repair.hygiene_finding_count, 1);
    assert_eq!(
        plan.projected_after_repair
            .hygiene_quarantine_candidate_count,
        1
    );
    assert!(plan.listed_repairs.is_empty());
}

#[test]
fn admission_hygiene_penalizes_cross_task_transcript_pollution() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Conversation transcript: user: rust for loop assistant: here is code".to_owned(),
        lesson: "ssh -o ConnectTimeout=5 gitlab.local merge_requests bash command".to_owned(),
        quality: 0.93,
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("polluted admission", 0.93)
    });

    let record = store
        .records()
        .iter()
        .find(|record| record.id == polluted_id)
        .unwrap();

    assert!(record.quality <= 0.05);
    assert!(record.process_reward.total <= 0.05);
    assert_eq!(record.process_reward.action, RewardAction::Penalize);
    assert!(
        record
            .process_reward
            .notes
            .iter()
            .any(|note| note.contains("experience_hygiene=cross_task_shell_transcript"))
    );
}

#[test]
fn persistence_admission_guard_skips_cross_task_transcript_pollution() {
    let path = temp_path("experience-admission-guard");
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "Conversation transcript: user: rust for loop assistant: here is code".to_owned(),
        lesson: "ssh -o ConnectTimeout=5 gitlab.local merge_requests bash command".to_owned(),
        quality: 0.93,
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        ..input("polluted persistence admission", 0.93)
    });
    let clean_id = store.record(ExperienceInput {
        prompt: "Rust for loop examples".to_owned(),
        lesson: "show a clean Rust range loop with for i in 0..10".to_owned(),
        quality: 0.82,
        ..input("clean rust persistence", 0.82)
    });

    let in_memory_hygiene = store.hygiene_report(8);
    store.save_to_disk_kv(&path).unwrap();
    let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

    assert_eq!(in_memory_hygiene.quarantine_candidate_count, 1);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.records()[0].id, clean_id);
    assert!(loaded.records()[0].lesson.contains("clean Rust range loop"));
    assert_eq!(loaded.hygiene_report(8).quarantine_candidate_count, 0);
    cleanup(path);
}

#[test]
fn read_only_load_missing_experience_does_not_create_state() {
    let path = temp_path("experience-read-only-missing");

    let loaded = ExperienceStore::load_from_disk_kv_read_only(&path).unwrap();

    assert!(loaded.is_empty());
    assert!(!path.exists());
    cleanup(path);
}

#[test]
fn read_only_load_preserves_partial_tail_record() {
    let path = temp_path("experience-read-only-partial-tail");
    let mut store = ExperienceStore::new();
    let id = store.record(input("stable read only load", 0.82));
    store.save_to_disk_kv(&path).unwrap();
    let clean_len = fs::metadata(&path).unwrap().len();

    {
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"NDK1").unwrap();
        file.write_all(&[1]).unwrap();
    }
    let dirty_len = fs::metadata(&path).unwrap().len();

    let loaded = ExperienceStore::load_from_disk_kv_read_only(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.records()[0].id, id);
    assert_eq!(fs::metadata(&path).unwrap().len(), dirty_len);
    assert!(dirty_len > clean_len);
    cleanup(path);
}

#[test]
fn hygiene_quarantine_split_moves_candidates_out_of_retained_store() {
    let mut store = ExperienceStore::new();
    let clean_id = store.record(ExperienceInput {
        prompt: "build a clean rust loop".to_owned(),
        lesson: "use for item in items without unrelated shell logs".to_owned(),
        ..input("clean", 0.89)
    });
    let polluted_id = store.record(ExperienceInput {
        prompt: "Conversation transcript: user: rust loop assistant: sample".to_owned(),
        lesson: "ssh -o ConnectTimeout=5 gitlab.local merge_requests bash command".to_owned(),
        ..input("polluted quarantine", 0.91)
    });

    let (retained, quarantined, plan) = store.split_hygiene_quarantine(8);

    assert_eq!(plan.total_records, 2);
    assert_eq!(plan.retained_records, 1);
    assert_eq!(plan.quarantine_candidate_count, 1);
    assert_eq!(plan.candidate_ids, vec![polluted_id]);
    assert_eq!(retained.records().len(), 1);
    assert_eq!(retained.records()[0].id, clean_id);
    assert_eq!(quarantined.records().len(), 1);
    assert_eq!(quarantined.records()[0].id, polluted_id);
}

#[test]
fn hygiene_quarantine_plan_limit_zero_keeps_candidate_ids_without_listed_findings() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "Conversation transcript: user: rust loop assistant: sample".to_owned(),
        lesson: "ssh -o ConnectTimeout=5 gitlab.local merge_requests bash command".to_owned(),
        ..input("polluted quarantine", 0.91)
    });

    let plan = store.hygiene_quarantine_plan(0);

    assert_eq!(plan.total_records, 1);
    assert_eq!(plan.retained_records, 0);
    assert_eq!(plan.quarantine_candidate_count, 1);
    assert_eq!(plan.candidate_ids, vec![polluted_id]);
    assert!(plan.listed_findings.is_empty());
}

#[test]
fn retrieval_exposes_runtime_diagnostics() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "adapter selection for local runtime".to_owned(),
        lesson: "reuse portable-rust runtime diagnostics".to_owned(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("noiron-runtime-v2".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            device_profile: Some("cpu".to_owned()),
            primary_lane: Some("cpu-vector".to_owned()),
            fallback_lane: Some("cpu-portable".to_owned()),
            memory_mode: Some("tiered-disk".to_owned()),
            device_execution_source: Some(
                RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
            ),
            layer_count: 16,
            global_layers: 4,
            local_window_layers: 8,
            convolutional_fusion_layers: 4,
            hidden_size: 128,
            local_window_tokens: 4096,
            forward_energy: Some(0.33),
            kv_influence: Some(0.44),
            imported_kv_blocks: 2,
            exported_kv_blocks: 3,
            hot_kv_precision_bits: Some(8),
            cold_kv_precision_bits: Some(4),
            ..RuntimeDiagnostics::default()
        },
        ..input("runtime", 0.9)
    });

    let matches = store.retrieve_lessons("portable-rust adapter", TaskProfile::Coding, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].runtime_model_id.as_deref(),
        Some("noiron-runtime-v2")
    );
    assert_eq!(
        matches[0].runtime_selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(matches[0].runtime_device_profile.as_deref(), Some("cpu"));
    assert_eq!(
        matches[0].runtime_primary_lane.as_deref(),
        Some("cpu-vector")
    );
    assert_eq!(
        matches[0].runtime_fallback_lane.as_deref(),
        Some("cpu-portable")
    );
    assert_eq!(
        matches[0].runtime_memory_mode.as_deref(),
        Some("tiered-disk")
    );
    assert_eq!(
        matches[0].runtime_device_execution_source.as_deref(),
        Some(RuntimeDiagnostics::runtime_reported_device_execution_source())
    );
    assert_eq!(matches[0].runtime_forward_energy, Some(0.33));
    assert_eq!(matches[0].runtime_kv_influence, Some(0.44));
    assert_eq!(matches[0].runtime_uncertainty_perplexity, Some(4.38));
    assert_eq!(matches[0].used_memory_count, 2);
    assert_eq!(matches[0].stored_runtime_kv_memory_ids, vec![11]);
    assert_eq!(matches[0].route_threshold, 0.55);
    assert_eq!(matches[0].route_attention_tokens, 2);
    assert_eq!(matches[0].route_fast_tokens, 3);
    assert_eq!(matches[0].route_attention_fraction, 0.4);

    let hint = render_experience_hint(&matches[0]);
    assert!(hint.contains("used_memories=2"), "{hint}");
    assert!(hint.contains("route_attention_fraction=0.400"), "{hint}");
    assert!(hint.contains("route_attention_tokens=2"), "{hint}");
}

#[test]
fn retrieval_drops_untrusted_runtime_selected_adapter() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "adapter selection should sanitize polluted runtime metadata".to_owned(),
        lesson: "do not expose unknown runtime adapter names".to_owned(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("noiron-runtime-v2".to_owned()),
            selected_adapter: Some("unknown-adapter secret=sk-retrieval-leak".to_owned()),
            forward_energy: Some(0.33),
            kv_influence: Some(0.44),
            ..RuntimeDiagnostics::default()
        },
        ..input("runtime", 0.9)
    });

    let matches = store.retrieve_lessons("runtime adapter metadata", TaskProfile::Coding, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].runtime_model_id.as_deref(),
        Some("noiron-runtime-v2")
    );
    assert_eq!(matches[0].runtime_selected_adapter, None);
}

#[test]
fn retrieval_signal_text_ignores_mutated_untrusted_runtime_selected_adapter() {
    let mut store = ExperienceStore::new();
    let polluted_id = store.record(ExperienceInput {
        prompt: "plain scheduler prompt".to_owned(),
        profile: TaskProfile::Writing,
        lesson: "plain scheduler lesson".to_owned(),
        quality: 0.0,
        runtime_diagnostics: RuntimeDiagnostics {
            selected_adapter: Some("portable-rust".to_owned()),
            ..RuntimeDiagnostics::default()
        },
        ..input("mutated runtime adapter signal", 0.0)
    });
    store
        .record_mut(polluted_id)
        .unwrap()
        .runtime_diagnostics
        .selected_adapter = Some("未知适配器 钥匙泄漏标记".to_owned());

    let polluted_matches = store.retrieve_lessons("钥匙泄漏标记", TaskProfile::Coding, 5);

    assert!(polluted_matches.is_empty());
}

#[test]
fn legacy_runtime_diagnostics_deserialize_without_kv_precision() {
    let legacy = [
        "model",
        "portable-rust",
        "cpu",
        "cpu-vector",
        "cpu-portable",
        "tiered-disk",
        "8",
        "2",
        "4",
        "2",
        "64",
        "2048",
        "0.250000",
        "0.750000",
        "1",
        "2",
    ]
    .join("\u{1f}");

    let diagnostics = deserialize_runtime_diagnostics(&legacy).unwrap();

    assert_eq!(diagnostics.model_id.as_deref(), Some("model"));
    assert_eq!(diagnostics.hot_kv_precision_bits, None);
    assert_eq!(diagnostics.cold_kv_precision_bits, None);
    assert_eq!(diagnostics.device_execution_source, None);
    assert_eq!(diagnostics.runtime_kv_segment_count(), 0);
    assert!(!diagnostics.has_valid_kv_precision_signal());
}

#[test]
fn runtime_diagnostics_roundtrip_preserves_device_execution_source() {
    let mut diagnostics = RuntimeDiagnostics::default()
        .with_device_execution("cpu", "cpu-vector", "cpu-portable", "tiered-disk")
        .with_kv_precision(8, 4);
    diagnostics.adapter_cache_mode = Some("genome_filtered".to_owned());
    diagnostics.adapter_stream_trace_id = Some("trace-runtime-38".to_owned());
    diagnostics.adapter_stream_gate_summary_digest = Some("fnv64:0123456789abcdef".to_owned());
    diagnostics.adapter_stream_read_only = Some(true);
    diagnostics.adapter_stream_write_allowed = Some(false);
    diagnostics.adapter_stream_applied = Some(false);
    diagnostics.runtime_kv_segments_included = 2;
    diagnostics.runtime_kv_segments_skipped = 1;
    diagnostics.runtime_kv_segments_rejected = 1;
    diagnostics.weak_runtime_kv_imports_skipped = 3;
    diagnostics.budget_limited_runtime_kv_imports_skipped = 4;

    let encoded = serialize_runtime_diagnostics(&diagnostics);
    let decoded = deserialize_runtime_diagnostics(&encoded).unwrap();

    assert_eq!(
        decoded.device_execution_source.as_deref(),
        Some(RuntimeDiagnostics::runtime_reported_device_execution_source())
    );
    assert!(decoded.has_runtime_reported_device_execution_signal());
    assert_eq!(
        decoded.adapter_cache_mode.as_deref(),
        Some("genome_filtered")
    );
    assert_eq!(
        decoded.adapter_stream_trace_id.as_deref(),
        Some("trace-runtime-38")
    );
    assert_eq!(
        decoded.adapter_stream_gate_summary_digest.as_deref(),
        Some("fnv64:0123456789abcdef")
    );
    assert!(decoded.has_adapter_stream_trace_signal());
    assert!(decoded.has_adapter_stream_gate_summary_signal());
    assert!(decoded.has_adapter_stream_write_gate_signal());
    assert_eq!(decoded.adapter_stream_preview_only(), Some(true));
    assert_eq!(decoded.runtime_kv_segments_included, 2);
    assert_eq!(decoded.runtime_kv_segments_skipped, 1);
    assert_eq!(decoded.runtime_kv_segments_rejected, 1);
    assert_eq!(decoded.weak_runtime_kv_imports_skipped, 3);
    assert_eq!(decoded.budget_limited_runtime_kv_imports_skipped, 4);
    assert_eq!(decoded.runtime_kv_segment_count(), 4);
}

#[test]
fn retrieval_exposes_recursive_runtime_calls_from_reward_notes() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "long document recursive runtime".to_owned(),
        lesson: "expensive recursive runtime calls should be reusable control feedback".to_owned(),
        process_reward: ProcessRewardReport {
            total: 0.77,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "Recursive:Chunks= 8 :Merge_Rounds=2:Waves=4:Parallel=2:Runtime_Calls= 13 "
                    .to_owned(),
            ],
        },
        ..input("recursive runtime", 0.9)
    });

    let matches = store.retrieve_lessons("runtime_calls", TaskProfile::LongDocument, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].recursive_runtime_calls, Some(13));
}

#[test]
fn retrieval_exposes_recursive_runtime_calls_from_full_width_reward_notes() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "long document recursive runtime".to_owned(),
        lesson: "recursive runtime calls survive imported full-width evidence notes".to_owned(),
        process_reward: ProcessRewardReport {
            total: 0.77,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "Recursive：Chunks＝ 8 ：Merge_Rounds＝2：Waves＝4：Parallel＝2：Runtime_Calls＝ 13 "
                    .to_owned(),
            ],
        },
        ..input("full width recursive runtime", 0.9)
    });

    let matches = store.retrieve_lessons("runtime_calls", TaskProfile::LongDocument, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].recursive_runtime_calls, Some(13));
}

#[test]
fn retrieval_uses_max_recursive_runtime_calls_from_multiple_notes() {
    let mut store = ExperienceStore::new();
    store.record(ExperienceInput {
        prompt: "long document recursive runtime".to_owned(),
        lesson: "recursive runtime calls should keep the strongest observed schedule evidence"
            .to_owned(),
        process_reward: ProcessRewardReport {
            total: 0.77,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "recursive:chunks=4:runtime_calls=7".to_owned(),
                "recursive:chunks=8:runtime_calls=13".to_owned(),
                "latency:recursive_runtime_calls=21".to_owned(),
            ],
        },
        ..input("recursive runtime max", 0.9)
    });

    let matches = store.retrieve_lessons("runtime_calls", TaskProfile::LongDocument, 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].recursive_runtime_calls, Some(13));
}

fn input(lesson: &str, quality: f32) -> ExperienceInput {
    ExperienceInput {
        prompt: "build a Noiron loop".to_owned(),
            profile: TaskProfile::Coding,
            lesson: lesson.to_owned(),
            quality,
            contradictions: Vec::new(),
            reflection_issues: vec![ReflectionIssue::new(
                "needs_grounding",
                ReflectionSeverity::Warning,
                "needs grounding detail",
            )],
            revision_actions: vec!["revise_reflection_signal".to_owned()],
            stored_memory_id: Some(42),
            router_threshold_after: 0.55,
            stream_windows: 3,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 2,
                fast_tokens: 3,
                attention_fraction: 0.4,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![3, 5],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: vec![11],
            runtime_token_metrics: ExperienceRuntimeTokenMetrics {
                token_count: 3,
                entropy_count: 3,
                logprob_count: 2,
                average_entropy: Some(0.42),
                average_neg_logprob: Some(0.70),
                uncertainty_perplexity: Some(4.38),
            },
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("noiron-test-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                device_execution_source: Some(
                    RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
                ),
                layer_count: 8,
                global_layers: 2,
                local_window_layers: 4,
                convolutional_fusion_layers: 2,
                hidden_size: 64,
                local_window_tokens: 2048,
                forward_energy: Some(0.25),
                kv_influence: Some(0.75),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
                ..RuntimeDiagnostics::default()
            },
            process_reward: ProcessRewardReport {
                notes: vec![
                    "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.820000:penalty_amount=0.000000"
                        .to_owned(),
                ],
                ..ProcessRewardReport::default()
            },
            live_evolution: LiveInferenceEvolution {
                router_threshold_delta: 0.03,
                hierarchy_weight_delta: 0.04,
                online_reward_feedbacks: 1,
                online_reward_reinforcements: 1,
                online_reward_penalties: 0,
                online_reward_strength: 0.72,
                online_reward_reinforcement_strength: 0.72,
                online_reward_penalty_strength: 0.0,
                memory_reinforcements: 1,
                memory_penalties: 0,
                stored_memory: true,
                stored_gist_memories: 2,
                stored_runtime_kv_memories: 1,
                reflection_issues: 1,
                critical_reflection_issues: 0,
                revision_actions: 1,
            },
        }
}

fn gist(summary: &str, level: GistLevel, importance: f32) -> GistRecord {
    GistRecord {
        level,
        title: "gist title".to_owned(),
        summary: summary.to_owned(),
        source_tokens: 8,
        importance,
    }
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}.ndkv",
        std::process::id()
    ))
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_file(path);
}
