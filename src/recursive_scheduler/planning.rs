use super::schedule::{RecursiveExecutionWave, RecursiveMergeRound};

pub(super) fn plan_merge_rounds(chunks: usize, fan_in: usize) -> Vec<RecursiveMergeRound> {
    let mut rounds = Vec::new();
    let mut input_units = chunks;
    let fan_in = fan_in.max(2);

    while input_units > 1 {
        let output_units = input_units.div_ceil(fan_in);
        rounds.push(RecursiveMergeRound {
            round: rounds.len(),
            input_units,
            output_units,
        });
        input_units = output_units;
    }

    rounds
}

pub(super) fn plan_execution_waves(
    chunks: usize,
    max_parallel_chunks: usize,
) -> Vec<RecursiveExecutionWave> {
    let max_parallel_chunks = max_parallel_chunks.max(1);
    let mut waves = Vec::new();
    let mut start_chunk = 0;

    while start_chunk < chunks {
        let end_chunk = (start_chunk + max_parallel_chunks).min(chunks);
        waves.push(RecursiveExecutionWave {
            wave: waves.len(),
            start_chunk,
            end_chunk,
            chunk_count: end_chunk - start_chunk,
        });
        start_chunk = end_chunk;
    }

    waves
}

pub(super) fn estimate_prompt_tokens(prompt: &str) -> usize {
    if prompt.trim().is_empty() {
        return 0;
    }

    let word_count = prompt.split_whitespace().count();
    if prompt.chars().any(char::is_whitespace) {
        word_count
    } else {
        let divisor = if prompt.is_ascii() { 4 } else { 2 };
        prompt.chars().count().div_ceil(divisor).max(1)
    }
}
