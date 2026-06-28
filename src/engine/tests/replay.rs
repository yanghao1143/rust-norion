use super::*;

#[test]
fn replay_experience_reinforces_rewarded_memory() {
    let mut engine = NoironEngine::new();
    let memory_id = engine
        .cache
        .store_or_fuse("replay memory", vec![1.0, 0.0, 0.0], 0.8);
    engine.experience.record(ExperienceInput {
        prompt: "Rust replay router".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "reinforce high reward control path".to_owned(),
        quality: 0.92,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(memory_id),
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.91,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    let before_hits = engine.cache.entries()[0].hits;

    let report = engine.replay_experience(4);

    assert_eq!(report.applied, 1);
    assert_eq!(report.router_updates, 1);
    assert_eq!(report.hierarchy_updates, 1);
    assert_eq!(report.reinforced, 1);
    assert_eq!(report.memory_reinforcements, 1);
    assert!(engine.cache.entries()[0].hits > before_hits);
    assert!(engine.router.observations() > 0);
    assert_eq!(engine.evolution_ledger.replay_runs, 1);
    assert_eq!(engine.evolution_ledger.replay_items, 1);
    assert_eq!(engine.evolution_ledger.memory_reinforcements, 1);
    assert_eq!(engine.evolution_ledger.memory_penalties, 0);
}

#[test]
fn replay_experience_reinforces_used_memory_ids() {
    let mut engine = NoironEngine::new();
    let memory_id = engine
        .cache
        .store_or_fuse("used replay memory", vec![1.0, 0.0, 0.0], 0.8);
    engine.experience.record(ExperienceInput {
        prompt: "Rust replay used memory".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "reinforce memories that helped a high reward answer".to_owned(),
        quality: 0.93,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    let before_hits = engine.cache.entries()[0].hits;

    let report = engine.replay_experience(4);

    assert_eq!(report.touched_memories, 1);
    assert_eq!(report.memory_reinforcements, 1);
    assert!(engine.cache.entries()[0].hits > before_hits);
    assert_eq!(engine.evolution_ledger.replay_runs, 1);
    assert_eq!(engine.evolution_ledger.memory_updates(), 1);
}

#[test]
fn replay_experience_skips_hygiene_quarantine_candidates() {
    let mut engine = NoironEngine::new();
    let polluted_memory_id =
        engine
            .cache
            .store_or_fuse("polluted replay memory", vec![1.0, 0.0, 0.0], 0.8);
    let clean_memory_id =
        engine
            .cache
            .store_or_fuse("clean replay memory", vec![0.0, 1.0, 0.0], 0.8);

    let polluted_experience_id = engine.experience.record(replay_memory_input(
        "Conversation transcript: user: rust loop assistant: sample",
        "ssh -o ConnectTimeout=5 gitlab.local merge_requests bash command",
        0.91,
        polluted_memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    let polluted_record = engine
        .experience
        .record_mut(polluted_experience_id)
        .unwrap();
    polluted_record.quality = 0.99;
    polluted_record.process_reward.total = 0.99;
    polluted_record.process_reward.action = RewardAction::Reinforce;

    engine.experience.record(replay_memory_input(
        "clean replay path",
        "reinforce a clean reusable rust memory",
        0.80,
        clean_memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    let polluted_before = memory_strength(&engine, polluted_memory_id);
    let clean_before = memory_strength(&engine, clean_memory_id);
    let quarantine_plan = engine.experience.hygiene_quarantine_plan(8);

    let report = engine.replay_experience(1);

    assert_eq!(quarantine_plan.candidate_ids, vec![polluted_experience_id]);
    assert_eq!(report.planned, 1);
    assert_eq!(report.applied, 1);
    assert_eq!(report.reinforced, 1);
    assert_eq!(report.memory_reinforcements, 1);
    assert_eq!(engine.evolution_ledger.replay_items, 1);
    assert_eq!(engine.evolution_ledger.memory_updates(), 1);
    assert_eq!(
        memory_strength(&engine, polluted_memory_id),
        polluted_before
    );
    assert!(memory_strength(&engine, clean_memory_id) > clean_before);
    assert!(!report
        .notes
        .iter()
        .any(|note| note.contains(&format!("experience:{polluted_experience_id}:"))));
}

#[test]
fn replay_experience_scales_penalties_from_reflection_diagnostics() {
    let mut engine = NoironEngine::new();
    let plain_memory_id =
        engine
            .cache
            .store_or_fuse("plain replay penalty", vec![1.0, 0.0, 0.0], 0.9);
    let diagnosed_memory_id =
        engine
            .cache
            .store_or_fuse("diagnosed replay penalty", vec![0.0, 1.0, 0.0], 0.9);
    engine.experience.record(replay_memory_input(
        "plain penalty path",
        "penalize weak memory without diagnostics",
        0.30,
        plain_memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "diagnosed penalty path",
        "penalize weak memory with critical reflection repair",
        0.30,
        diagnosed_memory_id,
        vec![ReflectionIssue::new(
            "unsupported_claim",
            ReflectionSeverity::Critical,
            "critical reflection issue should increase memory penalty",
        )],
        vec!["rerun local verification before reuse".to_owned()],
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));

    let report = engine.replay_experience(4);

    assert_eq!(report.penalized, 2);
    assert_eq!(report.memory_penalties, 2);
    assert!(
        memory_strength(&engine, diagnosed_memory_id) < memory_strength(&engine, plain_memory_id)
    );
    assert!(report.notes.iter().any(|note| {
        note.contains("memory_update=0.950")
            && note.contains("critical=1")
            && note.contains("actions=1")
    }));
}

#[test]
fn replay_experience_scales_reinforcement_from_runtime_and_recursive_cost() {
    let mut engine = NoironEngine::new();
    let plain_memory_id =
        engine
            .cache
            .store_or_fuse("plain replay reinforcement", vec![1.0, 0.0, 0.0], 0.8);
    let runtime_memory_id =
        engine
            .cache
            .store_or_fuse("runtime replay reinforcement", vec![0.0, 1.0, 0.0], 0.8);
    let expensive_memory_id = engine.cache.store_or_fuse(
        "expensive recursive replay reinforcement",
        vec![0.0, 0.0, 1.0],
        0.8,
    );
    engine.experience.record(replay_memory_input(
        "plain reinforcement path",
        "reinforce useful memory without runtime diagnostics",
        0.80,
        plain_memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "runtime reinforcement path",
        "reinforce useful memory with imported KV influence",
        0.80,
        runtime_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_diagnostics(0.80),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "expensive recursive reinforcement path",
        "dampen useful memory when recursive runtime cost is excessive",
        0.80,
        expensive_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_diagnostics(0.80),
        vec!["recursive:chunks=4:merge_rounds=2:waves=2:parallel=1:runtime_calls=96".to_owned()],
    ));

    let report = engine.replay_experience(4);

    assert_eq!(report.reinforced, 3);
    assert_eq!(report.memory_reinforcements, 3);
    assert!(
        memory_strength(&engine, runtime_memory_id) > memory_strength(&engine, plain_memory_id)
    );
    assert!(
        memory_strength(&engine, expensive_memory_id) < memory_strength(&engine, plain_memory_id)
    );
    assert!(report.notes.iter().any(|note| {
        note.contains("memory_update=0.793") && note.contains("recursive_runtime_calls=96")
    }));
}

#[test]
fn replay_experience_downweights_reinforcement_from_runtime_kv_budget_pressure() {
    let mut engine = NoironEngine::new();
    let efficient_memory_id = engine.cache.store_or_fuse(
        "efficient runtime replay reinforcement",
        vec![1.0, 0.0, 0.0],
        0.8,
    );
    let pressured_memory_id = engine.cache.store_or_fuse(
        "pressured runtime replay reinforcement",
        vec![0.0, 1.0, 0.0],
        0.8,
    );

    engine.experience.record(replay_memory_input(
        "efficient runtime kv budget replay",
        "reinforce runtime kv memory when budget pressure is absent",
        0.80,
        efficient_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_budget_diagnostics(0.80, 0),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "pressured runtime kv budget replay",
        "dampen runtime kv memory when budget skips dominated replay evidence",
        0.80,
        pressured_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_budget_diagnostics(0.80, 4),
        Vec::new(),
    ));

    let report = engine.replay_experience(4);

    assert_eq!(report.memory_reinforcements, 2);
    assert!(
        memory_strength(&engine, efficient_memory_id)
            > memory_strength(&engine, pressured_memory_id)
    );
    assert!(report.notes.iter().any(|note| {
        note.contains("memory_update=0.816") && note.contains("runtime_kv_budget_pressure=0.800")
    }));
}

#[test]
fn replay_experience_increases_penalty_from_runtime_kv_budget_pressure() {
    let mut engine = NoironEngine::new();
    let efficient_memory_id =
        engine
            .cache
            .store_or_fuse("efficient runtime replay penalty", vec![1.0, 0.0, 0.0], 0.9);
    let pressured_memory_id =
        engine
            .cache
            .store_or_fuse("pressured runtime replay penalty", vec![0.0, 1.0, 0.0], 0.9);

    engine.experience.record(replay_memory_input(
        "efficient runtime kv penalty replay",
        "penalize weak runtime memory without budget pressure",
        0.30,
        efficient_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_budget_diagnostics(0.20, 0),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "pressured runtime kv penalty replay",
        "penalize weak runtime memory harder when budget skips dominate",
        0.30,
        pressured_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_budget_diagnostics(0.20, 4),
        Vec::new(),
    ));

    let report = engine.replay_experience(4);

    assert_eq!(report.memory_penalties, 2);
    assert!(
        memory_strength(&engine, pressured_memory_id)
            < memory_strength(&engine, efficient_memory_id)
    );
    assert!(report.notes.iter().any(|note| {
        note.contains("memory_update=0.780") && note.contains("runtime_kv_budget_pressure=0.800")
    }));
}

#[test]
fn replay_experience_uses_runtime_kv_segment_quality_for_reinforcement() {
    let mut engine = NoironEngine::new();
    let included_memory_id =
        engine
            .cache
            .store_or_fuse("included kv segment replay", vec![1.0, 0.0, 0.0], 0.8);
    let rejected_memory_id =
        engine
            .cache
            .store_or_fuse("rejected kv segment replay", vec![0.0, 1.0, 0.0], 0.8);

    engine.experience.record(replay_memory_input(
        "included runtime kv segment path",
        "reinforce memory with accepted runtime kv segments",
        0.80,
        included_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_segment_diagnostics(0.80, 3, 0, 0),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
        "rejected runtime kv segment path",
        "dampen memory when runtime kv segments are rejected",
        0.80,
        rejected_memory_id,
        Vec::new(),
        Vec::new(),
        replay_runtime_segment_diagnostics(0.80, 0, 0, 3),
        Vec::new(),
    ));

    let report = engine.replay_experience(4);

    assert_eq!(report.memory_reinforcements, 2);
    assert!(
        memory_strength(&engine, included_memory_id) > memory_strength(&engine, rejected_memory_id)
    );
}

#[test]
fn replay_experience_consumes_structured_live_evolution_for_update_strength() {
    let mut engine = NoironEngine::new();
    let plain_reinforce_id =
        engine
            .cache
            .store_or_fuse("plain live evolution reinforce", vec![1.0, 0.0, 0.0], 0.8);
    let evolved_reinforce_id =
        engine
            .cache
            .store_or_fuse("evolved live evolution reinforce", vec![0.0, 1.0, 0.0], 0.8);
    let plain_penalty_id =
        engine
            .cache
            .store_or_fuse("plain live evolution penalty", vec![0.0, 0.0, 1.0], 0.9);
    let evolved_penalty_id =
        engine
            .cache
            .store_or_fuse("evolved live evolution penalty", vec![0.5, 0.5, 0.0], 0.9);

    engine.experience.record(replay_memory_input(
        "plain structured reinforce",
        "reinforce without structured live evolution evidence",
        0.80,
        plain_reinforce_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine
        .experience
        .record(replay_memory_input_with_live_evolution(
            "evolved structured reinforce",
            "reinforce with successful live evolution evidence",
            0.80,
            evolved_reinforce_id,
            LiveInferenceEvolution {
                router_threshold_delta: 0.18,
                hierarchy_weight_delta: 0.08,
                online_reward_feedbacks: 1,
                online_reward_reinforcements: 1,
                online_reward_penalties: 0,
                online_reward_strength: 0.90,
                online_reward_reinforcement_strength: 0.90,
                online_reward_penalty_strength: 0.0,
                memory_reinforcements: 3,
                memory_penalties: 0,
                stored_memory: true,
                stored_gist_memories: 1,
                stored_runtime_kv_memories: 1,
                reflection_issues: 0,
                critical_reflection_issues: 0,
                revision_actions: 0,
            },
        ));
    engine.experience.record(replay_memory_input(
        "plain structured penalty",
        "penalize without structured live evolution evidence",
        0.30,
        plain_penalty_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine
        .experience
        .record(replay_memory_input_with_live_evolution(
            "evolved structured penalty",
            "penalize with failed live evolution evidence",
            0.30,
            evolved_penalty_id,
            LiveInferenceEvolution {
                router_threshold_delta: 0.0,
                hierarchy_weight_delta: 0.0,
                online_reward_feedbacks: 1,
                online_reward_reinforcements: 0,
                online_reward_penalties: 1,
                online_reward_strength: 0.85,
                online_reward_reinforcement_strength: 0.0,
                online_reward_penalty_strength: 0.85,
                memory_reinforcements: 0,
                memory_penalties: 3,
                stored_memory: false,
                stored_gist_memories: 0,
                stored_runtime_kv_memories: 0,
                reflection_issues: 3,
                critical_reflection_issues: 1,
                revision_actions: 2,
            },
        ));

    let report = engine.replay_experience(8);

    assert_eq!(report.applied, 4);
    assert_eq!(report.live_evolution_items, 2);
    assert_eq!(report.live_evolution_memory_updates, 6);
    assert_eq!(engine.evolution_ledger.replay_live_evolution_items, 2);
    assert_eq!(
        engine.evolution_ledger.replay_live_evolution_memory_updates,
        6
    );
    assert!(
        memory_strength(&engine, evolved_reinforce_id)
            > memory_strength(&engine, plain_reinforce_id)
    );
    assert!(
        memory_strength(&engine, evolved_penalty_id) < memory_strength(&engine, plain_penalty_id)
    );
}

#[test]
fn replay_experience_uses_live_memory_feedback_notes() {
    let mut engine = NoironEngine::new();
    let plain_memory_id =
        engine
            .cache
            .store_or_fuse("plain live feedback replay", vec![1.0, 0.0, 0.0], 0.8);
    let live_memory_id =
        engine
            .cache
            .store_or_fuse("boosted live feedback replay", vec![0.0, 1.0, 0.0], 0.8);
    let live_penalty_memory_id =
        engine
            .cache
            .store_or_fuse("penalized live feedback replay", vec![0.0, 0.0, 1.0], 0.8);
    engine.experience.record(replay_memory_input(
        "plain live feedback reinforcement",
        "reinforce without online memory evidence",
        0.80,
        plain_memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine.experience.record(replay_memory_input(
            "boosted live feedback reinforcement",
            "reinforce memory with online reinforcement evidence",
            0.80,
            live_memory_id,
            Vec::new(),
            Vec::new(),
            RuntimeDiagnostics::default(),
            vec![
                "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.900000:penalty_amount=0.000000"
                    .to_owned(),
            ],
        ));
    engine.experience.record(replay_memory_input(
            "penalized live feedback path",
            "penalize memory with online penalty evidence",
            0.30,
            live_penalty_memory_id,
            Vec::new(),
            Vec::new(),
            RuntimeDiagnostics::default(),
            vec![
                "memory_feedback:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.900000"
                    .to_owned(),
            ],
        ));

    let report = engine.replay_experience(4);

    assert_eq!(report.memory_reinforcements, 2);
    assert_eq!(report.memory_penalties, 1);
    assert_eq!(report.touched_memories, 3);
    assert_eq!(report.applied_memory_updates, 3);
    assert_eq!(report.missing_memory_updates, 0);
    assert_eq!(report.memory_update_reports.len(), 3);
    assert!(report.memory_strength_delta > 0.0);
    assert!(memory_strength(&engine, live_memory_id) > memory_strength(&engine, plain_memory_id));
    assert!(memory_strength(&engine, live_penalty_memory_id) < 0.62);
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("memory_update=0.872")));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("memory_update=0.862")));
}

#[test]
fn replay_experience_notes_do_not_expose_lesson_text() {
    let mut engine = NoironEngine::new();
    let memory_id =
        engine
            .cache
            .store_or_fuse("replay note redaction memory", vec![1.0, 0.0, 0.0], 0.8);
    let raw_lesson = "raw replay lesson should stay out";
    engine.experience.record(replay_memory_input(
        "replay note redaction prompt",
        raw_lesson,
        0.90,
        memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));

    let report = engine.replay_experience(4);
    let joined = report.notes.join("\n");

    assert!(joined.contains(&format!("lesson_chars={}", raw_lesson.chars().count())));
    assert!(!joined.contains("lesson="));
    assert!(!joined.contains(raw_lesson));
}
