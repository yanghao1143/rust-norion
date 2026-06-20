use super::values::{parse_f32, parse_u64, parse_usize};

pub(crate) struct StateFlagParse<'a> {
    pub(crate) native_window_tokens: &'a mut usize,
    pub(crate) chunk_tokens: &'a mut usize,
    pub(crate) chunk_overlap_tokens: &'a mut usize,
    pub(crate) merge_fan_in: &'a mut usize,
    pub(crate) replay_limit: &'a mut usize,
    pub(crate) auto_replay_limit: &'a mut usize,
    pub(crate) retention_stale_after: &'a mut Option<u64>,
    pub(crate) retention_decay_rate: &'a mut Option<f32>,
    pub(crate) retention_remove_below: &'a mut Option<f32>,
    pub(crate) retention_remove_after_failures: &'a mut Option<u64>,
    pub(crate) compaction_similarity_threshold: &'a mut Option<f32>,
    pub(crate) compaction_max_candidates: &'a mut Option<usize>,
    pub(crate) compaction_max_merges: &'a mut Option<usize>,
}

impl StateFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        match raw.get(index)?.as_str() {
            "--native-window" => {
                let value = raw.get(index + 1)?;
                *self.native_window_tokens = parse_usize(value, *self.native_window_tokens);
                Some(2)
            }
            "--chunk-tokens" => {
                let value = raw.get(index + 1)?;
                *self.chunk_tokens = parse_usize(value, *self.chunk_tokens);
                Some(2)
            }
            "--chunk-overlap" => {
                let value = raw.get(index + 1)?;
                *self.chunk_overlap_tokens = parse_usize(value, *self.chunk_overlap_tokens);
                Some(2)
            }
            "--merge-fan-in" => {
                let value = raw.get(index + 1)?;
                *self.merge_fan_in = parse_usize(value, *self.merge_fan_in);
                Some(2)
            }
            "--replay" => {
                let value = raw.get(index + 1)?;
                *self.replay_limit = parse_usize(value, *self.replay_limit);
                Some(2)
            }
            "--auto-replay" => {
                let value = raw.get(index + 1)?;
                *self.auto_replay_limit = parse_usize(value, *self.auto_replay_limit);
                Some(2)
            }
            "--retention-stale-after" => {
                let value = raw.get(index + 1)?;
                *self.retention_stale_after = Some(parse_u64(value, 64));
                Some(2)
            }
            "--retention-decay-rate" => {
                let value = raw.get(index + 1)?;
                *self.retention_decay_rate = Some(parse_f32(value, 0.04));
                Some(2)
            }
            "--retention-remove-below" => {
                let value = raw.get(index + 1)?;
                *self.retention_remove_below = Some(parse_f32(value, 0.04));
                Some(2)
            }
            "--retention-remove-after-failures" => {
                let value = raw.get(index + 1)?;
                *self.retention_remove_after_failures = Some(parse_u64(value, 4));
                Some(2)
            }
            "--compaction-threshold" => {
                let value = raw.get(index + 1)?;
                *self.compaction_similarity_threshold = Some(parse_f32(value, 0.92));
                Some(2)
            }
            "--compaction-max-candidates" => {
                let value = raw.get(index + 1)?;
                *self.compaction_max_candidates = Some(parse_usize(value, 512));
                Some(2)
            }
            "--compaction-max-merges" => {
                let value = raw.get(index + 1)?;
                *self.compaction_max_merges = Some(parse_usize(value, 32));
                Some(2)
            }
            _ => None,
        }
    }
}
