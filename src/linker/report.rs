use super::{CandidateBackend, SemanticLinkReport};

impl CandidateBackend {
    /// Stable backend identifier stored in reports and edge evidence.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::WeavatrixVector => "weavatrix_search_vector",
        }
    }

    /// Whether candidate coverage is exhaustive.
    #[must_use]
    pub const fn is_exact(self) -> bool {
        matches!(self, Self::Exact)
    }
}

impl SemanticLinkReport {
    /// Number of input vectors.
    #[must_use]
    pub const fn vector_count(&self) -> usize {
        self.vector_count
    }

    /// Shared embedding dimension, or zero for an empty input.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Exact eligible-pair cosine comparisons performed by exhaustive linking.
    #[must_use]
    pub const fn comparisons(&self) -> u64 {
        self.comparisons
    }

    /// Number of distinct unordered endpoint pairs represented by the edges.
    #[must_use]
    pub const fn pair_count(&self) -> usize {
        self.pair_count
    }

    /// Backend used to generate semantic candidates.
    #[must_use]
    pub const fn candidate_backend(&self) -> CandidateBackend {
        self.candidate_backend
    }

    /// Stable eligibility policy identifier.
    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// Number of emitted directed graph edges.
    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Directed semantic graph edges.
    #[must_use]
    pub fn edges(&self) -> &[weavatrix_graph::Edge] {
        &self.edges
    }

    /// Consumes the report and returns its graph edges.
    #[must_use]
    pub fn into_edges(self) -> Vec<weavatrix_graph::Edge> {
        self.edges
    }
}
