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

        RecursiveSchedule {
            prompt_tokens,
            native_window_tokens: self.native_window_tokens,
            chunk_tokens: self.chunk_tokens,
            overlap_tokens: self.overlap_tokens,
            merge_fan_in: self.merge_fan_in,
            chunks,
            merge_rounds,
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

#[derive(Debug, Clone)]
pub struct RecursiveSchedule {
    pub prompt_tokens: usize,
    pub native_window_tokens: usize,
    pub chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub merge_fan_in: usize,
    pub chunks: Vec<RecursiveChunk>,
    pub merge_rounds: Vec<RecursiveMergeRound>,
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

    pub fn summary(&self) -> String {
        format!(
            "required={} prompt_tokens={} native_window={} chunks={} merge_rounds={} chunk_tokens={} overlap_tokens={} merge_fan_in={}",
            self.requires_recursion,
            self.prompt_tokens,
            self.native_window_tokens,
            self.chunk_count(),
            self.merge_round_count(),
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

fn plan_merge_rounds(chunks: usize, fan_in: usize) -> Vec<RecursiveMergeRound> {
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

fn estimate_prompt_tokens(prompt: &str) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_prompt_stays_single_pass() {
        let scheduler = RecursiveScheduler::new(8, 6, 2, 2);

        let schedule = scheduler.plan("one two three");

        assert!(!schedule.requires_recursion);
        assert_eq!(schedule.prompt_tokens, 3);
        assert_eq!(schedule.chunk_count(), 1);
        assert_eq!(schedule.merge_round_count(), 0);
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
}
