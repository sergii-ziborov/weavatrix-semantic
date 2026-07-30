mod edge;
mod execute;
pub(crate) mod math;
mod report;

use crate::LinkConfig;
use weavatrix_graph::{Edge, EdgeKind};

/// Custom graph edge kind emitted by this crate.
pub const SEMANTIC_EDGE_KIND: &str = "semantic_similarity";

/// Stable provenance extractor identity used for idempotent relinking.
pub const SEMANTIC_EXTRACTOR: &str = "weavatrix-semantic";

/// Strategy used to generate candidate neighborhoods before semantic policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateBackend {
    /// Exhaustive all-pairs cosine comparisons.
    Exact,
    /// First-party `weavatrix-search-vector` HNSW candidates.
    WeavatrixVector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Candidate {
    pub(crate) target: usize,
    pub(crate) score: f64,
}

/// Summary of one semantic-linking run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticLinkReport {
    vector_count: usize,
    dimension: usize,
    comparisons: u64,
    pair_count: usize,
    candidate_backend: CandidateBackend,
    policy_id: String,
    edges: Vec<Edge>,
}

/// Exact cosine top-K semantic linker for existing graph nodes.
#[derive(Debug, Clone)]
pub struct SemanticLinker {
    config: LinkConfig,
    edge_kind: EdgeKind,
}
