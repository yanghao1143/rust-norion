mod model;

pub use model::{
    GeneScissorsIntent, GenomeExpression, GenomeExpressionInput, MutationPlan, ReasoningGene,
    ReasoningGeneKind, ReasoningGeneStatus, ReasoningGenome,
};

#[cfg(test)]
mod tests;
