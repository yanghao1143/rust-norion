use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn max_recursive_chunks(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_chunks)
            .max()
            .unwrap_or(0)
    }

    pub fn recursive_cases(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.requires_recursion)
            .count()
    }

    pub fn max_recursive_waves(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_waves)
            .max()
            .unwrap_or(0)
    }

    pub fn total_recursive_runtime_calls(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.recursive_runtime_calls)
            .sum()
    }
}
