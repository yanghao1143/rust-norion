use super::planning::plan_execution_waves;
use super::scheduler::RecursiveScheduler;

#[derive(Debug, Clone)]
pub struct RecursiveSchedule {
    pub prompt_tokens: usize,
    pub native_window_tokens: usize,
    pub chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub merge_fan_in: usize,
    pub max_parallel_chunks: usize,
    pub chunks: Vec<RecursiveChunk>,
    pub merge_rounds: Vec<RecursiveMergeRound>,
    pub execution_waves: Vec<RecursiveExecutionWave>,
    pub requires_recursion: bool,
}

impl Default for RecursiveSchedule {
    fn default() -> Self {
        RecursiveScheduler::default().plan("")
    }
}

impl RecursiveSchedule {
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn merge_round_count(&self) -> usize {
        self.merge_rounds.len()
    }

    pub fn execution_wave_count(&self) -> usize {
        self.execution_waves.len()
    }

    pub fn with_parallel_budget(mut self, max_parallel_chunks: usize) -> Self {
        self.max_parallel_chunks = max_parallel_chunks.max(1);
        self.execution_waves = plan_execution_waves(
            if self.requires_recursion {
                self.chunks.len()
            } else {
                usize::from(!self.chunks.is_empty())
            },
            self.max_parallel_chunks,
        );
        self
    }

    pub fn summary(&self) -> String {
        format!(
            "required={} prompt_tokens={} native_window={} chunks={} merge_rounds={} execution_waves={} max_parallel_chunks={} chunk_tokens={} overlap_tokens={} merge_fan_in={}",
            self.requires_recursion,
            self.prompt_tokens,
            self.native_window_tokens,
            self.chunk_count(),
            self.merge_round_count(),
            self.execution_wave_count(),
            self.max_parallel_chunks,
            self.chunk_tokens,
            self.overlap_tokens,
            self.merge_fan_in
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveChunk {
    pub index: usize,
    pub start_token: usize,
    pub end_token: usize,
    pub estimated_tokens: usize,
    pub overlap_left: usize,
    pub overlap_right: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveMergeRound {
    pub round: usize,
    pub input_units: usize,
    pub output_units: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveExecutionWave {
    pub wave: usize,
    pub start_chunk: usize,
    pub end_chunk: usize,
    pub chunk_count: usize,
}
