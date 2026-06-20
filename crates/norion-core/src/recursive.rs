#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveSchedulerConfig {
    pub native_window_tokens: usize,
    pub chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub merge_fan_in: usize,
}

impl RecursiveSchedulerConfig {
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

    pub fn plan_prompt(self, prompt: &str) -> RecursiveScheduleDigest {
        self.plan_tokens(estimate_prompt_tokens(prompt))
    }

    pub fn plan_tokens(self, prompt_tokens: usize) -> RecursiveScheduleDigest {
        let requires_recursion = prompt_tokens > self.native_window_tokens;
        let chunks = self.plan_chunks(prompt_tokens);
        let merge_rounds = if requires_recursion {
            plan_merge_rounds(chunks.len(), self.merge_fan_in)
        } else {
            Vec::new()
        };
        let execution_waves = plan_execution_waves(chunks.len(), 1);

        RecursiveScheduleDigest {
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

    fn plan_chunks(self, prompt_tokens: usize) -> Vec<RecursiveChunk> {
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

impl Default for RecursiveSchedulerConfig {
    fn default() -> Self {
        Self::new(8_192, 4_096, 256, 4)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveScheduleDigest {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveScheduleSummary {
    pub requires_recursion: bool,
    pub prompt_tokens: usize,
    pub native_window_tokens: usize,
    pub chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub merge_fan_in: usize,
    pub max_parallel_chunks: usize,
    pub chunk_count: usize,
    pub merge_round_count: usize,
    pub execution_wave_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveScheduleAction {
    UseRecursiveSchedule,
    WaitForRecursiveSchedule,
    RepairRecursiveSchedule,
}

impl RecursiveScheduleAction {
    pub fn can_use(self) -> bool {
        matches!(self, Self::UseRecursiveSchedule)
    }

    pub fn should_wait(self) -> bool {
        matches!(self, Self::WaitForRecursiveSchedule)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairRecursiveSchedule)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveScheduleValidationSummary {
    pub valid: bool,
    pub violation_count: usize,
    pub shape_violation_count: usize,
    pub chunk_violation_count: usize,
    pub merge_violation_count: usize,
    pub execution_wave_violation_count: usize,
}

impl RecursiveScheduleValidationSummary {
    pub fn has_shape_failures(self) -> bool {
        self.shape_violation_count > 0
    }

    pub fn has_chunk_failures(self) -> bool {
        self.chunk_violation_count > 0
    }

    pub fn has_merge_failures(self) -> bool {
        self.merge_violation_count > 0
    }

    pub fn has_execution_wave_failures(self) -> bool {
        self.execution_wave_violation_count > 0
    }

    pub fn has_payload_failures(self) -> bool {
        self.has_chunk_failures() || self.has_merge_failures() || self.has_execution_wave_failures()
    }

    pub fn shape_failure_component_count(self) -> usize {
        usize::from(self.has_shape_failures())
    }

    pub fn chunk_failure_component_count(self) -> usize {
        usize::from(self.has_chunk_failures())
    }

    pub fn merge_failure_component_count(self) -> usize {
        usize::from(self.has_merge_failures())
    }

    pub fn execution_wave_failure_component_count(self) -> usize {
        usize::from(self.has_execution_wave_failures())
    }

    pub fn payload_failure_component_count(self) -> usize {
        usize::from(self.has_payload_failures())
    }

    pub fn validation_problem_component_count(self) -> usize {
        self.shape_failure_component_count()
            .saturating_add(self.chunk_failure_component_count())
            .saturating_add(self.merge_failure_component_count())
            .saturating_add(self.execution_wave_failure_component_count())
    }

    pub fn has_validation_problem_components(self) -> bool {
        self.validation_problem_component_count() > 0
    }

    pub fn violation_count_matches_parts(self) -> bool {
        self.violation_count
            == self
                .shape_violation_count
                .saturating_add(self.chunk_violation_count)
                .saturating_add(self.merge_violation_count)
                .saturating_add(self.execution_wave_violation_count)
    }

    pub fn validation_component_accounting_is_consistent(self) -> bool {
        self.validation_problem_component_count()
            == self
                .shape_failure_component_count()
                .saturating_add(self.chunk_failure_component_count())
                .saturating_add(self.merge_failure_component_count())
                .saturating_add(self.execution_wave_failure_component_count())
    }

    pub fn is_clean_validation(self) -> bool {
        self.valid
            && self.violation_count == 0
            && !self.has_validation_problem_components()
            && self.violation_count_matches_parts()
    }

    pub fn validation_shape_is_clean(self) -> bool {
        self.is_clean_validation() && self.validation_component_accounting_is_consistent()
    }

    pub fn can_accept_recursive_schedule_validation(self) -> bool {
        self.validation_shape_is_clean()
    }
}

impl RecursiveScheduleSummary {
    pub fn from_digest(digest: &RecursiveScheduleDigest) -> Self {
        Self {
            requires_recursion: digest.requires_recursion,
            prompt_tokens: digest.prompt_tokens,
            native_window_tokens: digest.native_window_tokens,
            chunk_tokens: digest.chunk_tokens,
            overlap_tokens: digest.overlap_tokens,
            merge_fan_in: digest.merge_fan_in,
            max_parallel_chunks: digest.max_parallel_chunks,
            chunk_count: digest.chunk_count(),
            merge_round_count: digest.merge_round_count(),
            execution_wave_count: digest.execution_wave_count(),
        }
    }

    pub fn contract_violations(&self, request_prompt_tokens: usize) -> Vec<String> {
        let mut violations = Vec::new();

        if self.prompt_tokens != request_prompt_tokens {
            violations.push(format!(
                "recursive schedule prompt_tokens {} differ from request prompt_tokens {}",
                self.prompt_tokens, request_prompt_tokens
            ));
        }
        if self.native_window_tokens == 0 {
            violations.push(
                "recursive schedule native_window_tokens must be greater than zero".to_owned(),
            );
        }
        if self.chunk_tokens == 0 {
            violations.push("recursive schedule chunk_tokens must be greater than zero".to_owned());
        }
        if self.overlap_tokens >= self.chunk_tokens && self.chunk_tokens > 0 {
            violations.push(format!(
                "recursive schedule overlap_tokens {} must be below chunk_tokens {}",
                self.overlap_tokens, self.chunk_tokens
            ));
        }
        if self.merge_fan_in < 2 {
            violations.push("recursive schedule merge_fan_in must be at least 2".to_owned());
        }
        if self.max_parallel_chunks == 0 {
            violations.push(
                "recursive schedule max_parallel_chunks must be greater than zero".to_owned(),
            );
        }
        if self.requires_recursion {
            if self.chunk_count <= 1 {
                violations.push(
                    "recursive schedule requires recursion but has fewer than 2 chunks".to_owned(),
                );
            }
            if self.merge_round_count == 0 {
                violations.push(
                    "recursive schedule requires recursion but has no merge rounds".to_owned(),
                );
            }
        } else if self.merge_round_count > 0 {
            violations.push("recursive schedule has merge rounds without recursion".to_owned());
        }
        if self.chunk_count > 0 && self.execution_wave_count == 0 {
            violations.push("recursive schedule has chunks but no execution waves".to_owned());
        }

        violations
    }

    pub fn validation_summary(
        &self,
        request_prompt_tokens: usize,
    ) -> RecursiveScheduleValidationSummary {
        let mut shape_violation_count = 0;
        let chunk_violation_count = 0;
        let mut merge_violation_count = 0;
        let mut execution_wave_violation_count = 0;

        if self.prompt_tokens != request_prompt_tokens {
            shape_violation_count += 1;
        }
        if self.native_window_tokens == 0 {
            shape_violation_count += 1;
        }
        if self.chunk_tokens == 0 {
            shape_violation_count += 1;
        }
        if self.overlap_tokens >= self.chunk_tokens && self.chunk_tokens > 0 {
            shape_violation_count += 1;
        }
        if self.merge_fan_in < 2 {
            shape_violation_count += 1;
        }
        if self.max_parallel_chunks == 0 {
            shape_violation_count += 1;
        }
        if self.requires_recursion && self.chunk_count <= 1 {
            shape_violation_count += 1;
        }
        if self.requires_recursion && self.merge_round_count == 0 {
            merge_violation_count += 1;
        }
        if !self.requires_recursion && self.merge_round_count > 0 {
            merge_violation_count += 1;
        }
        if self.chunk_count > 0 && self.execution_wave_count == 0 {
            execution_wave_violation_count += 1;
        }

        recursive_validation_summary(
            shape_violation_count,
            chunk_violation_count,
            merge_violation_count,
            execution_wave_violation_count,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.chunk_count == 0
            && self.merge_round_count == 0
            && self.execution_wave_count == 0
            && self.prompt_tokens == 0
    }

    pub fn is_single_pass(&self) -> bool {
        !self.requires_recursion && self.chunk_count <= 1 && self.merge_round_count == 0
    }

    pub fn parallelism_was_requested(&self) -> bool {
        self.max_parallel_chunks > 1
    }

    pub fn has_chunk_work(&self) -> bool {
        self.chunk_count > 0
    }

    pub fn has_merge_work(&self) -> bool {
        self.merge_round_count > 0
    }

    pub fn has_execution_waves(&self) -> bool {
        self.execution_wave_count > 0
    }

    pub fn scheduler_shape_is_valid(&self) -> bool {
        self.native_window_tokens > 0
            && self.chunk_tokens > 0
            && self.overlap_tokens < self.chunk_tokens
            && self.merge_fan_in >= 2
            && self.max_parallel_chunks > 0
    }

    pub fn recursion_shape_is_valid(&self) -> bool {
        if self.requires_recursion {
            self.chunk_count > 1 && self.merge_round_count > 0
        } else {
            self.merge_round_count == 0
        }
    }

    pub fn execution_wave_shape_is_valid(&self) -> bool {
        self.chunk_count == 0 || self.execution_wave_count > 0
    }

    pub fn recursion_activity_signal_component_count(&self) -> usize {
        usize::from(self.requires_recursion)
    }

    pub fn chunk_activity_signal_component_count(&self) -> usize {
        usize::from(self.has_chunk_work())
    }

    pub fn merge_activity_signal_component_count(&self) -> usize {
        usize::from(self.has_merge_work())
    }

    pub fn execution_wave_signal_component_count(&self) -> usize {
        usize::from(self.has_execution_waves())
    }

    pub fn parallelism_signal_component_count(&self) -> usize {
        usize::from(self.parallelism_was_requested())
    }

    pub fn schedule_signal_component_count(&self) -> usize {
        self.recursion_activity_signal_component_count()
            .saturating_add(self.chunk_activity_signal_component_count())
            .saturating_add(self.merge_activity_signal_component_count())
            .saturating_add(self.execution_wave_signal_component_count())
            .saturating_add(self.parallelism_signal_component_count())
    }

    pub fn has_schedule_signals(&self) -> bool {
        self.schedule_signal_component_count() > 0
    }

    pub fn scheduler_shape_problem_component_count(&self) -> usize {
        usize::from(self.native_window_tokens == 0)
            .saturating_add(usize::from(self.chunk_tokens == 0))
            .saturating_add(usize::from(
                self.chunk_tokens > 0 && self.overlap_tokens >= self.chunk_tokens,
            ))
            .saturating_add(usize::from(self.merge_fan_in < 2))
            .saturating_add(usize::from(self.max_parallel_chunks == 0))
    }

    pub fn recursion_shape_problem_component_count(&self) -> usize {
        usize::from(self.requires_recursion && self.chunk_count <= 1)
            .saturating_add(usize::from(
                self.requires_recursion && self.merge_round_count == 0,
            ))
            .saturating_add(usize::from(
                !self.requires_recursion && self.merge_round_count > 0,
            ))
    }

    pub fn execution_wave_problem_component_count(&self) -> usize {
        usize::from(!self.execution_wave_shape_is_valid())
    }

    pub fn schedule_shape_problem_component_count(&self) -> usize {
        self.scheduler_shape_problem_component_count()
            .saturating_add(self.recursion_shape_problem_component_count())
            .saturating_add(self.execution_wave_problem_component_count())
    }

    pub fn has_schedule_shape_problem_components(&self) -> bool {
        self.schedule_shape_problem_component_count() > 0
    }

    pub fn schedule_accounting_is_consistent(&self) -> bool {
        let expected_signal_count = usize::from(self.requires_recursion)
            .saturating_add(usize::from(self.has_chunk_work()))
            .saturating_add(usize::from(self.has_merge_work()))
            .saturating_add(usize::from(self.has_execution_waves()))
            .saturating_add(usize::from(self.parallelism_was_requested()));
        let expected_problem_count = usize::from(self.native_window_tokens == 0)
            .saturating_add(usize::from(self.chunk_tokens == 0))
            .saturating_add(usize::from(
                self.chunk_tokens > 0 && self.overlap_tokens >= self.chunk_tokens,
            ))
            .saturating_add(usize::from(self.merge_fan_in < 2))
            .saturating_add(usize::from(self.max_parallel_chunks == 0))
            .saturating_add(usize::from(
                self.requires_recursion && self.chunk_count <= 1,
            ))
            .saturating_add(usize::from(
                self.requires_recursion && self.merge_round_count == 0,
            ))
            .saturating_add(usize::from(
                !self.requires_recursion && self.merge_round_count > 0,
            ))
            .saturating_add(usize::from(!self.execution_wave_shape_is_valid()));

        self.schedule_signal_component_count() == expected_signal_count
            && self.schedule_shape_problem_component_count() == expected_problem_count
            && self.has_schedule_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn schedule_shape_is_clean(&self) -> bool {
        !self.has_schedule_shape_problem_components() && self.schedule_accounting_is_consistent()
    }

    pub fn can_use_recursive_schedule(&self) -> bool {
        self.has_chunk_work() && self.schedule_shape_is_clean()
    }

    pub fn recursive_schedule_action(&self) -> RecursiveScheduleAction {
        if self.can_use_recursive_schedule() {
            RecursiveScheduleAction::UseRecursiveSchedule
        } else if self.has_schedule_shape_problem_components() {
            RecursiveScheduleAction::RepairRecursiveSchedule
        } else {
            RecursiveScheduleAction::WaitForRecursiveSchedule
        }
    }

    pub fn minimum_runtime_units(&self) -> usize {
        self.chunk_count.saturating_add(self.merge_round_count)
    }

    pub fn minimum_recursion_overhead_units(&self) -> usize {
        if self.requires_recursion {
            self.merge_round_count
        } else {
            0
        }
    }
}

impl RecursiveScheduleDigest {
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn merge_round_count(&self) -> usize {
        self.merge_rounds.len()
    }

    pub fn execution_wave_count(&self) -> usize {
        self.execution_waves.len()
    }

    pub fn chunk_runtime_units(&self) -> usize {
        self.chunks.len()
    }

    pub fn merge_runtime_units(&self) -> usize {
        self.merge_rounds
            .iter()
            .map(|round| round.output_units)
            .sum()
    }

    pub fn total_runtime_units(&self) -> usize {
        self.chunk_runtime_units()
            .saturating_add(self.merge_runtime_units())
    }

    pub fn recursion_overhead_units(&self) -> usize {
        if self.requires_recursion {
            self.merge_runtime_units()
        } else {
            0
        }
    }

    pub fn max_execution_wave_width(&self) -> usize {
        self.execution_waves
            .iter()
            .map(|wave| wave.chunk_count)
            .max()
            .unwrap_or(0)
    }

    pub fn parallelism_was_used(&self) -> bool {
        self.max_execution_wave_width() > 1
    }

    pub fn is_empty(&self) -> bool {
        self.prompt_tokens == 0
            && self.chunks.is_empty()
            && self.merge_rounds.is_empty()
            && self.execution_waves.is_empty()
    }

    pub fn is_single_pass(&self) -> bool {
        !self.requires_recursion && self.chunks.len() <= 1 && self.merge_rounds.is_empty()
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

    pub fn contract_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();

        if self.native_window_tokens == 0 {
            violations.push(
                "recursive schedule native_window_tokens must be greater than zero".to_owned(),
            );
        }
        if self.chunk_tokens == 0 {
            violations.push("recursive schedule chunk_tokens must be greater than zero".to_owned());
        }
        if self.chunk_tokens > self.native_window_tokens && self.native_window_tokens > 0 {
            violations.push(format!(
                "recursive schedule chunk_tokens {} exceed native_window_tokens {}",
                self.chunk_tokens, self.native_window_tokens
            ));
        }
        if self.overlap_tokens >= self.chunk_tokens && self.chunk_tokens > 0 {
            violations.push(format!(
                "recursive schedule overlap_tokens {} must be below chunk_tokens {}",
                self.overlap_tokens, self.chunk_tokens
            ));
        }
        if self.merge_fan_in < 2 {
            violations.push("recursive schedule merge_fan_in must be at least 2".to_owned());
        }
        if self.max_parallel_chunks == 0 {
            violations.push(
                "recursive schedule max_parallel_chunks must be greater than zero".to_owned(),
            );
        }
        if self.requires_recursion && self.chunks.len() <= 1 {
            violations.push(
                "recursive schedule requires recursion but has fewer than 2 chunks".to_owned(),
            );
        }
        if !self.requires_recursion && self.merge_round_count() > 0 {
            violations.push("recursive schedule has merge rounds without recursion".to_owned());
        }

        for (expected, chunk) in self.chunks.iter().enumerate() {
            if chunk.index != expected {
                violations.push(format!(
                    "recursive chunk index {} should be {}",
                    chunk.index, expected
                ));
            }
            if chunk.start_token >= chunk.end_token {
                violations.push(format!(
                    "recursive chunk {} token range {}..{} is empty or reversed",
                    chunk.index, chunk.start_token, chunk.end_token
                ));
            }
            if chunk.end_token > self.prompt_tokens {
                violations.push(format!(
                    "recursive chunk {} end_token {} exceeds prompt_tokens {}",
                    chunk.index, chunk.end_token, self.prompt_tokens
                ));
            }
            if chunk.estimated_tokens != chunk.end_token.saturating_sub(chunk.start_token) {
                violations.push(format!(
                    "recursive chunk {} estimated_tokens {} differ from token range length {}",
                    chunk.index,
                    chunk.estimated_tokens,
                    chunk.end_token.saturating_sub(chunk.start_token)
                ));
            }
            if chunk.overlap_left > chunk.estimated_tokens
                || chunk.overlap_right > chunk.estimated_tokens
            {
                violations.push(format!(
                    "recursive chunk {} overlap exceeds estimated tokens",
                    chunk.index
                ));
            }
        }

        for (expected, round) in self.merge_rounds.iter().enumerate() {
            if round.round != expected {
                violations.push(format!(
                    "recursive merge round {} should be {}",
                    round.round, expected
                ));
            }
            if round.input_units <= round.output_units && round.input_units > 1 {
                violations.push(format!(
                    "recursive merge round {} does not reduce units {} -> {}",
                    round.round, round.input_units, round.output_units
                ));
            }
        }

        for (expected, wave) in self.execution_waves.iter().enumerate() {
            if wave.wave != expected {
                violations.push(format!(
                    "recursive execution wave {} should be {}",
                    wave.wave, expected
                ));
            }
            if wave.start_chunk >= wave.end_chunk {
                violations.push(format!(
                    "recursive execution wave {} chunk range {}..{} is empty or reversed",
                    wave.wave, wave.start_chunk, wave.end_chunk
                ));
            }
            if wave.chunk_count != wave.end_chunk.saturating_sub(wave.start_chunk) {
                violations.push(format!(
                    "recursive execution wave {} chunk_count {} differs from range length {}",
                    wave.wave,
                    wave.chunk_count,
                    wave.end_chunk.saturating_sub(wave.start_chunk)
                ));
            }
            if wave.chunk_count > self.max_parallel_chunks {
                violations.push(format!(
                    "recursive execution wave {} chunk_count {} exceeds max_parallel_chunks {}",
                    wave.wave, wave.chunk_count, self.max_parallel_chunks
                ));
            }
        }

        violations
    }

    pub fn validation_summary(&self) -> RecursiveScheduleValidationSummary {
        let mut shape_violation_count = 0;
        let mut chunk_violation_count = 0;
        let mut merge_violation_count = 0;
        let mut execution_wave_violation_count = 0;

        if self.native_window_tokens == 0 {
            shape_violation_count += 1;
        }
        if self.chunk_tokens == 0 {
            shape_violation_count += 1;
        }
        if self.chunk_tokens > self.native_window_tokens && self.native_window_tokens > 0 {
            shape_violation_count += 1;
        }
        if self.overlap_tokens >= self.chunk_tokens && self.chunk_tokens > 0 {
            shape_violation_count += 1;
        }
        if self.merge_fan_in < 2 {
            shape_violation_count += 1;
        }
        if self.max_parallel_chunks == 0 {
            shape_violation_count += 1;
        }
        if self.requires_recursion && self.chunks.len() <= 1 {
            shape_violation_count += 1;
        }
        if !self.requires_recursion && self.merge_round_count() > 0 {
            merge_violation_count += 1;
        }

        for (expected, chunk) in self.chunks.iter().enumerate() {
            if chunk.index != expected {
                chunk_violation_count += 1;
            }
            if chunk.start_token >= chunk.end_token {
                chunk_violation_count += 1;
            }
            if chunk.end_token > self.prompt_tokens {
                chunk_violation_count += 1;
            }
            if chunk.estimated_tokens != chunk.end_token.saturating_sub(chunk.start_token) {
                chunk_violation_count += 1;
            }
            if chunk.overlap_left > chunk.estimated_tokens
                || chunk.overlap_right > chunk.estimated_tokens
            {
                chunk_violation_count += 1;
            }
        }

        for (expected, round) in self.merge_rounds.iter().enumerate() {
            if round.round != expected {
                merge_violation_count += 1;
            }
            if round.input_units <= round.output_units && round.input_units > 1 {
                merge_violation_count += 1;
            }
        }

        for (expected, wave) in self.execution_waves.iter().enumerate() {
            if wave.wave != expected {
                execution_wave_violation_count += 1;
            }
            if wave.start_chunk >= wave.end_chunk {
                execution_wave_violation_count += 1;
            }
            if wave.chunk_count != wave.end_chunk.saturating_sub(wave.start_chunk) {
                execution_wave_violation_count += 1;
            }
            if wave.chunk_count > self.max_parallel_chunks {
                execution_wave_violation_count += 1;
            }
        }

        recursive_validation_summary(
            shape_violation_count,
            chunk_violation_count,
            merge_violation_count,
            execution_wave_violation_count,
        )
    }

    pub fn is_valid(&self) -> bool {
        self.contract_violations().is_empty()
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

    pub fn schedule_summary(&self) -> RecursiveScheduleSummary {
        RecursiveScheduleSummary::from_digest(self)
    }
}

fn recursive_validation_summary(
    shape_violation_count: usize,
    chunk_violation_count: usize,
    merge_violation_count: usize,
    execution_wave_violation_count: usize,
) -> RecursiveScheduleValidationSummary {
    let violation_count = shape_violation_count
        .saturating_add(chunk_violation_count)
        .saturating_add(merge_violation_count)
        .saturating_add(execution_wave_violation_count);

    RecursiveScheduleValidationSummary {
        valid: violation_count == 0,
        violation_count,
        shape_violation_count,
        chunk_violation_count,
        merge_violation_count,
        execution_wave_violation_count,
    }
}

impl Default for RecursiveScheduleDigest {
    fn default() -> Self {
        RecursiveSchedulerConfig::default().plan_prompt("")
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

pub fn estimate_prompt_tokens(prompt: &str) -> usize {
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

fn plan_execution_waves(chunks: usize, max_parallel_chunks: usize) -> Vec<RecursiveExecutionWave> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_prompt_creates_single_non_recursive_chunk() {
        let schedule = RecursiveSchedulerConfig::new(8, 6, 2, 2).plan_prompt("one two three");

        assert!(!schedule.requires_recursion);
        assert_eq!(schedule.prompt_tokens, 3);
        assert_eq!(schedule.chunk_count(), 1);
        assert_eq!(schedule.merge_round_count(), 0);
        assert_eq!(schedule.execution_wave_count(), 1);
        assert_eq!(schedule.chunks[0].start_token, 0);
        assert_eq!(schedule.chunks[0].end_token, 3);
        assert!(schedule.is_valid());
    }

    #[test]
    fn long_prompt_creates_overlapping_chunks_and_merge_rounds() {
        let schedule = RecursiveSchedulerConfig::new(8, 6, 2, 2)
            .plan_tokens(14)
            .with_parallel_budget(2);

        assert!(schedule.requires_recursion);
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
        assert!(schedule.summary().contains("required=true"));
        assert!(schedule.is_valid());
    }

    #[test]
    fn token_estimation_matches_root_style_ascii_and_non_ascii_paths() {
        assert_eq!(estimate_prompt_tokens("one two three"), 3);
        assert_eq!(estimate_prompt_tokens("abcdefgh"), 2);
        assert_eq!(estimate_prompt_tokens("没有空格的长文本"), 4);
        assert_eq!(estimate_prompt_tokens("   "), 0);
    }

    #[test]
    fn scheduler_config_clamps_overlap_below_chunk_size() {
        let config = RecursiveSchedulerConfig::new(16, 4, 8, 1);

        assert_eq!(config.native_window_tokens, 16);
        assert_eq!(config.chunk_tokens, 4);
        assert_eq!(config.overlap_tokens, 3);
        assert_eq!(config.merge_fan_in, 2);
    }

    #[test]
    fn recursive_schedule_contract_reports_invalid_shapes() {
        let schedule = RecursiveScheduleDigest {
            prompt_tokens: 4,
            native_window_tokens: 0,
            chunk_tokens: 2,
            overlap_tokens: 2,
            merge_fan_in: 1,
            max_parallel_chunks: 1,
            chunks: vec![RecursiveChunk {
                index: 1,
                start_token: 3,
                end_token: 3,
                estimated_tokens: 1,
                overlap_left: 2,
                overlap_right: 0,
            }],
            merge_rounds: vec![RecursiveMergeRound {
                round: 1,
                input_units: 2,
                output_units: 2,
            }],
            execution_waves: vec![RecursiveExecutionWave {
                wave: 2,
                start_chunk: 1,
                end_chunk: 1,
                chunk_count: 2,
            }],
            requires_recursion: true,
        };

        let joined = schedule.contract_violations().join("\n");
        let summary = schedule.validation_summary();
        let schedule_summary = schedule.schedule_summary();

        assert!(joined.contains("native_window_tokens must be greater than zero"));
        assert!(joined.contains("overlap_tokens 2 must be below chunk_tokens 2"));
        assert!(joined.contains("merge_fan_in must be at least 2"));
        assert!(joined.contains("requires recursion but has fewer than 2 chunks"));
        assert!(joined.contains("recursive chunk index 1 should be 0"));
        assert!(joined.contains("token range 3..3 is empty or reversed"));
        assert!(joined.contains("estimated_tokens 1 differ"));
        assert!(joined.contains("merge round 1 should be 0"));
        assert!(joined.contains("does not reduce units 2 -> 2"));
        assert!(joined.contains("execution wave 2 should be 0"));
        assert!(joined.contains("chunk range 1..1 is empty or reversed"));
        assert!(joined.contains("chunk_count 2 differs from range length 0"));
        assert!(joined.contains("chunk_count 2 exceeds max_parallel_chunks 1"));
        assert!(!summary.valid);
        assert_eq!(
            summary.violation_count,
            schedule.contract_violations().len()
        );
        assert_eq!(summary.shape_violation_count, 4);
        assert_eq!(summary.chunk_violation_count, 4);
        assert_eq!(summary.merge_violation_count, 2);
        assert_eq!(summary.execution_wave_violation_count, 4);
        assert!(summary.has_shape_failures());
        assert!(summary.has_chunk_failures());
        assert!(summary.has_merge_failures());
        assert!(summary.has_execution_wave_failures());
        assert!(summary.has_payload_failures());
        assert_eq!(summary.shape_failure_component_count(), 1);
        assert_eq!(summary.chunk_failure_component_count(), 1);
        assert_eq!(summary.merge_failure_component_count(), 1);
        assert_eq!(summary.execution_wave_failure_component_count(), 1);
        assert_eq!(summary.payload_failure_component_count(), 1);
        assert_eq!(summary.validation_problem_component_count(), 4);
        assert!(summary.has_validation_problem_components());
        assert!(summary.violation_count_matches_parts());
        assert!(summary.validation_component_accounting_is_consistent());
        assert!(!summary.is_clean_validation());
        assert!(!summary.validation_shape_is_clean());
        assert!(!summary.can_accept_recursive_schedule_validation());
        assert!(schedule_summary.has_schedule_signals());
        assert_eq!(
            schedule_summary.recursion_activity_signal_component_count(),
            1
        );
        assert_eq!(schedule_summary.chunk_activity_signal_component_count(), 1);
        assert_eq!(schedule_summary.merge_activity_signal_component_count(), 1);
        assert_eq!(schedule_summary.execution_wave_signal_component_count(), 1);
        assert_eq!(schedule_summary.parallelism_signal_component_count(), 0);
        assert_eq!(schedule_summary.schedule_signal_component_count(), 4);
        assert!(!schedule_summary.scheduler_shape_is_valid());
        assert!(!schedule_summary.recursion_shape_is_valid());
        assert!(schedule_summary.execution_wave_shape_is_valid());
        assert_eq!(
            schedule_summary.scheduler_shape_problem_component_count(),
            3
        );
        assert_eq!(
            schedule_summary.recursion_shape_problem_component_count(),
            1
        );
        assert_eq!(schedule_summary.execution_wave_problem_component_count(), 0);
        assert_eq!(schedule_summary.schedule_shape_problem_component_count(), 4);
        assert!(schedule_summary.has_schedule_shape_problem_components());
        assert!(schedule_summary.schedule_accounting_is_consistent());
        assert!(!schedule_summary.schedule_shape_is_clean());
        assert!(!schedule_summary.can_use_recursive_schedule());
        assert_eq!(
            schedule_summary.recursive_schedule_action(),
            RecursiveScheduleAction::RepairRecursiveSchedule
        );
        assert!(!schedule_summary.recursive_schedule_action().can_use());
        assert!(!schedule_summary.recursive_schedule_action().should_wait());
        assert!(schedule_summary.recursive_schedule_action().should_repair());
    }

    #[test]
    fn recursive_schedule_summary_reports_request_mismatches() {
        let schedule = RecursiveSchedulerConfig::new(8, 6, 2, 2).plan_tokens(14);
        let summary = schedule.schedule_summary();

        assert!(summary.contract_violations(14).is_empty());
        assert!(!summary.is_empty());
        assert!(!summary.is_single_pass());
        assert!(summary.has_chunk_work());
        assert!(summary.has_merge_work());
        assert!(summary.has_execution_waves());
        assert!(!summary.parallelism_was_requested());
        assert!(summary.scheduler_shape_is_valid());
        assert!(summary.recursion_shape_is_valid());
        assert!(summary.execution_wave_shape_is_valid());
        assert_eq!(summary.recursion_activity_signal_component_count(), 1);
        assert_eq!(summary.chunk_activity_signal_component_count(), 1);
        assert_eq!(summary.merge_activity_signal_component_count(), 1);
        assert_eq!(summary.execution_wave_signal_component_count(), 1);
        assert_eq!(summary.parallelism_signal_component_count(), 0);
        assert_eq!(summary.schedule_signal_component_count(), 4);
        assert!(summary.has_schedule_signals());
        assert_eq!(summary.scheduler_shape_problem_component_count(), 0);
        assert_eq!(summary.recursion_shape_problem_component_count(), 0);
        assert_eq!(summary.execution_wave_problem_component_count(), 0);
        assert_eq!(summary.schedule_shape_problem_component_count(), 0);
        assert!(!summary.has_schedule_shape_problem_components());
        assert!(summary.schedule_accounting_is_consistent());
        assert!(summary.schedule_shape_is_clean());
        assert!(summary.can_use_recursive_schedule());
        assert_eq!(
            summary.recursive_schedule_action(),
            RecursiveScheduleAction::UseRecursiveSchedule
        );
        assert!(summary.recursive_schedule_action().can_use());
        assert!(!summary.recursive_schedule_action().should_wait());
        assert!(!summary.recursive_schedule_action().should_repair());
        let clean_validation = summary.validation_summary(14);
        assert!(clean_validation.valid);
        assert_eq!(clean_validation.violation_count, 0);
        assert_eq!(clean_validation.shape_failure_component_count(), 0);
        assert_eq!(clean_validation.chunk_failure_component_count(), 0);
        assert_eq!(clean_validation.merge_failure_component_count(), 0);
        assert_eq!(clean_validation.execution_wave_failure_component_count(), 0);
        assert_eq!(clean_validation.payload_failure_component_count(), 0);
        assert_eq!(clean_validation.validation_problem_component_count(), 0);
        assert!(!clean_validation.has_validation_problem_components());
        assert!(clean_validation.violation_count_matches_parts());
        assert!(clean_validation.validation_component_accounting_is_consistent());
        assert!(clean_validation.is_clean_validation());
        assert!(clean_validation.validation_shape_is_clean());
        assert!(clean_validation.can_accept_recursive_schedule_validation());

        let mut invalid = summary;
        invalid.prompt_tokens = 13;
        invalid.execution_wave_count = 0;

        let joined = invalid.contract_violations(14).join("\n");
        let validation = invalid.validation_summary(14);

        assert!(joined.contains("prompt_tokens 13 differ from request prompt_tokens 14"));
        assert!(joined.contains("has chunks but no execution waves"));
        assert!(!validation.valid);
        assert_eq!(validation.violation_count, 2);
        assert_eq!(validation.shape_violation_count, 1);
        assert_eq!(validation.chunk_violation_count, 0);
        assert_eq!(validation.merge_violation_count, 0);
        assert_eq!(validation.execution_wave_violation_count, 1);
        assert!(validation.has_shape_failures());
        assert!(!validation.has_chunk_failures());
        assert!(!validation.has_merge_failures());
        assert!(validation.has_execution_wave_failures());
        assert!(validation.has_payload_failures());
        assert_eq!(validation.shape_failure_component_count(), 1);
        assert_eq!(validation.chunk_failure_component_count(), 0);
        assert_eq!(validation.merge_failure_component_count(), 0);
        assert_eq!(validation.execution_wave_failure_component_count(), 1);
        assert_eq!(validation.payload_failure_component_count(), 1);
        assert_eq!(validation.validation_problem_component_count(), 2);
        assert!(validation.has_validation_problem_components());
        assert!(validation.violation_count_matches_parts());
        assert!(validation.validation_component_accounting_is_consistent());
        assert!(!validation.is_clean_validation());
        assert!(!validation.validation_shape_is_clean());
        assert!(!validation.can_accept_recursive_schedule_validation());
        assert!(!invalid.execution_wave_shape_is_valid());
        assert_eq!(invalid.execution_wave_problem_component_count(), 1);
        assert_eq!(invalid.schedule_shape_problem_component_count(), 1);
        assert!(invalid.has_schedule_shape_problem_components());
        assert!(invalid.schedule_accounting_is_consistent());
        assert!(!invalid.schedule_shape_is_clean());
        assert!(!invalid.can_use_recursive_schedule());
        assert_eq!(
            invalid.recursive_schedule_action(),
            RecursiveScheduleAction::RepairRecursiveSchedule
        );
        assert!(!invalid.recursive_schedule_action().can_use());
        assert!(!invalid.recursive_schedule_action().should_wait());
        assert!(invalid.recursive_schedule_action().should_repair());
    }

    #[test]
    fn recursive_runtime_unit_summary_counts_chunk_and_merge_work() {
        let schedule = RecursiveSchedulerConfig::new(8, 6, 2, 2)
            .plan_tokens(14)
            .with_parallel_budget(2);
        let summary = schedule.schedule_summary();

        assert_eq!(schedule.chunk_runtime_units(), 3);
        assert_eq!(schedule.merge_runtime_units(), 3);
        assert_eq!(schedule.total_runtime_units(), 6);
        assert_eq!(schedule.recursion_overhead_units(), 3);
        assert_eq!(schedule.max_execution_wave_width(), 2);
        assert!(schedule.parallelism_was_used());
        assert!(!schedule.is_single_pass());
        assert!(schedule.validation_summary().valid);

        assert_eq!(summary.minimum_runtime_units(), 5);
        assert_eq!(summary.minimum_recursion_overhead_units(), 2);
        assert!(summary.parallelism_was_requested());
        assert!(!summary.is_single_pass());
        assert!(!summary.is_empty());
    }

    #[test]
    fn recursive_runtime_unit_summary_marks_empty_and_single_pass_shapes() {
        let empty = RecursiveSchedulerConfig::new(8, 6, 2, 2).plan_tokens(0);
        let single = RecursiveSchedulerConfig::new(8, 6, 2, 2).plan_tokens(3);

        assert!(empty.is_empty());
        assert!(empty.is_single_pass());
        assert_eq!(empty.total_runtime_units(), 0);
        assert_eq!(empty.max_execution_wave_width(), 0);
        assert!(!empty.parallelism_was_used());

        let empty_summary = empty.schedule_summary();
        assert!(empty_summary.is_empty());
        assert!(empty_summary.is_single_pass());
        assert_eq!(empty_summary.minimum_runtime_units(), 0);
        assert_eq!(empty_summary.minimum_recursion_overhead_units(), 0);
        assert!(empty_summary.schedule_shape_is_clean());
        assert!(!empty_summary.can_use_recursive_schedule());
        assert_eq!(
            empty_summary.recursive_schedule_action(),
            RecursiveScheduleAction::WaitForRecursiveSchedule
        );
        assert!(!empty_summary.recursive_schedule_action().can_use());
        assert!(empty_summary.recursive_schedule_action().should_wait());
        assert!(!empty_summary.recursive_schedule_action().should_repair());

        assert!(!single.is_empty());
        assert!(single.is_single_pass());
        assert_eq!(single.chunk_runtime_units(), 1);
        assert_eq!(single.merge_runtime_units(), 0);
        assert_eq!(single.total_runtime_units(), 1);
        assert_eq!(single.recursion_overhead_units(), 0);

        let single_summary = single.schedule_summary();
        assert!(!single_summary.is_empty());
        assert!(single_summary.is_single_pass());
        assert_eq!(single_summary.minimum_runtime_units(), 1);
        assert_eq!(single_summary.minimum_recursion_overhead_units(), 0);
        assert!(single_summary.schedule_shape_is_clean());
        assert!(single_summary.can_use_recursive_schedule());
        assert_eq!(
            single_summary.recursive_schedule_action(),
            RecursiveScheduleAction::UseRecursiveSchedule
        );
        assert!(single_summary.recursive_schedule_action().can_use());
        assert!(!single_summary.recursive_schedule_action().should_wait());
        assert!(!single_summary.recursive_schedule_action().should_repair());
    }
}
