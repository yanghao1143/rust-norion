use super::*;

#[test]
fn short_prompt_stays_single_pass() {
    let scheduler = RecursiveScheduler::new(8, 6, 2, 2);

    let schedule = scheduler.plan("one two three");

    assert!(!schedule.requires_recursion);
    assert_eq!(schedule.prompt_tokens, 3);
    assert_eq!(schedule.chunk_count(), 1);
    assert_eq!(schedule.merge_round_count(), 0);
    assert_eq!(schedule.execution_wave_count(), 1);
    assert_eq!(schedule.chunks[0].start_token, 0);
    assert_eq!(schedule.chunks[0].end_token, 3);
}

#[test]
fn long_prompt_creates_overlapping_chunks_and_merge_rounds() {
    let scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");

    let schedule = scheduler.plan(&prompt);

    assert!(schedule.requires_recursion);
    assert_eq!(schedule.prompt_tokens, 14);
    assert_eq!(schedule.chunk_count(), 3);
    assert_eq!(
        schedule.chunks,
        vec![
            RecursiveChunk {
                index: 0,
                start_token: 0,
                end_token: 6,
                estimated_tokens: 6,
                overlap_left: 0,
                overlap_right: 2,
            },
            RecursiveChunk {
                index: 1,
                start_token: 4,
                end_token: 10,
                estimated_tokens: 6,
                overlap_left: 2,
                overlap_right: 2,
            },
            RecursiveChunk {
                index: 2,
                start_token: 8,
                end_token: 14,
                estimated_tokens: 6,
                overlap_left: 2,
                overlap_right: 0,
            },
        ]
    );
    assert_eq!(
        schedule.merge_rounds,
        vec![
            RecursiveMergeRound {
                round: 0,
                input_units: 3,
                output_units: 2,
            },
            RecursiveMergeRound {
                round: 1,
                input_units: 2,
                output_units: 1,
            },
        ]
    );
    assert_eq!(
        schedule.execution_waves,
        vec![
            RecursiveExecutionWave {
                wave: 0,
                start_chunk: 0,
                end_chunk: 1,
                chunk_count: 1,
            },
            RecursiveExecutionWave {
                wave: 1,
                start_chunk: 1,
                end_chunk: 2,
                chunk_count: 1,
            },
            RecursiveExecutionWave {
                wave: 2,
                start_chunk: 2,
                end_chunk: 3,
                chunk_count: 1,
            },
        ]
    );
}

#[test]
fn parallel_budget_groups_recursive_chunks_into_waves() {
    let scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");

    let schedule = scheduler.plan(&prompt).with_parallel_budget(2);

    assert_eq!(schedule.max_parallel_chunks, 2);
    assert_eq!(schedule.execution_wave_count(), 2);
    assert_eq!(
        schedule.execution_waves,
        vec![
            RecursiveExecutionWave {
                wave: 0,
                start_chunk: 0,
                end_chunk: 2,
                chunk_count: 2,
            },
            RecursiveExecutionWave {
                wave: 1,
                start_chunk: 2,
                end_chunk: 3,
                chunk_count: 1,
            },
        ]
    );
}

#[test]
fn overlap_is_clamped_below_chunk_size() {
    let scheduler = RecursiveScheduler::new(16, 4, 8, 2);

    assert_eq!(scheduler.chunk_tokens(), 4);
    assert_eq!(scheduler.overlap_tokens(), 3);
}

#[test]
fn no_whitespace_prompt_uses_character_fallback() {
    let scheduler = RecursiveScheduler::new(4, 3, 1, 2);

    let schedule = scheduler.plan("没有空格的长文本");

    assert_eq!(schedule.prompt_tokens, 4);
    assert!(!schedule.requires_recursion);
}
