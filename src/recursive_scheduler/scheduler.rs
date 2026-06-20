use super::planning::{estimate_prompt_tokens, plan_execution_waves, plan_merge_rounds};
use super::schedule::{RecursiveChunk, RecursiveSchedule};

#[derive(Debug, Clone)]
pub struct RecursiveScheduler {
    native_window_tokens: usize,
    chunk_tokens: usize,
    overlap_tokens: usize,
    merge_fan_in: usize,
}

impl Default for RecursiveScheduler {
    fn default() -> Self {
        Self {
            native_window_tokens: 8_192,
            chunk_tokens: 4_096,
            overlap_tokens: 256,
            merge_fan_in: 4,
        }
    }
}

impl RecursiveScheduler {
    pub fn new(
        native_window_tokens: usize,
        chunk_tokens: usize,
        overlap_tokens: usize,
        merge_fan_in: usize,
    ) -> Self {
        let native_window_tokens = native_window_tokens.max(1);
        let chunk_tokens = chunk_tokens.max(1).min(native_window_tokens);
        let overlap_tokens = overlap_tokens.min(chunk_tokens.saturating_sub(1));
        let merge_fan_in = merge_fan_in.max(2);

        Self {
            native_window_tokens,
            chunk_tokens,
            overlap_tokens,
            merge_fan_in,
        }
    }

    pub fn native_window_tokens(&self) -> usize {
        self.native_window_tokens
    }

    pub fn chunk_tokens(&self) -> usize {
        self.chunk_tokens
    }

    pub fn overlap_tokens(&self) -> usize {
        self.overlap_tokens
    }

    pub fn merge_fan_in(&self) -> usize {
        self.merge_fan_in
    }

    pub fn plan(&self, prompt: &str) -> RecursiveSchedule {
        let prompt_tokens = estimate_prompt_tokens(prompt);
        let requires_recursion = prompt_tokens > self.native_window_tokens;
        let chunks = self.plan_chunks(prompt_tokens);
        let merge_rounds = if requires_recursion {
            plan_merge_rounds(chunks.len(), self.merge_fan_in)
        } else {
            Vec::new()
        };
        let execution_waves = plan_execution_waves(chunks.len(), 1);

        RecursiveSchedule {
            prompt_tokens,
            native_window_tokens: self.native_window_tokens,
            chunk_tokens: self.chunk_tokens,
            overlap_tokens: self.overlap_tokens,
            merge_fan_in: self.merge_fan_in,
            max_parallel_chunks: 1,
            chunks,
            merge_rounds,
            execution_waves,
            requires_recursion,
        }
    }

    fn plan_chunks(&self, prompt_tokens: usize) -> Vec<RecursiveChunk> {
        if prompt_tokens == 0 {
            return Vec::new();
        }

        if prompt_tokens <= self.native_window_tokens {
            return vec![RecursiveChunk {
                index: 0,
                start_token: 0,
                end_token: prompt_tokens,
                estimated_tokens: prompt_tokens,
                overlap_left: 0,
                overlap_right: 0,
            }];
        }

        let stride = self.chunk_tokens.saturating_sub(self.overlap_tokens).max(1);
        let mut chunks = Vec::new();
        let mut start_token = 0;

        while start_token < prompt_tokens {
            let end_token = (start_token + self.chunk_tokens).min(prompt_tokens);
            let estimated_tokens = end_token.saturating_sub(start_token);
            let overlap_left = if chunks.is_empty() {
                0
            } else {
                self.overlap_tokens.min(estimated_tokens)
            };
            let overlap_right = if end_token < prompt_tokens {
                self.overlap_tokens.min(estimated_tokens)
            } else {
                0
            };

            chunks.push(RecursiveChunk {
                index: chunks.len(),
                start_token,
                end_token,
                estimated_tokens,
                overlap_left,
                overlap_right,
            });

            if end_token == prompt_tokens {
                break;
            }
            start_token += stride;
        }

        chunks
    }
}
