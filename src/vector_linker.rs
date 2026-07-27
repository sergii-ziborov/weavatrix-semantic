use crate::linker::Candidate;
use crate::linker::math::{cosine, ordered_vectors, retain_top_k};
use crate::{
    AllowAllPolicy, CandidateBackend, LinkConfig, LinkPolicy, Result, SemanticError,
    SemanticLinkReport, SemanticLinker, SemanticVector,
};
use weavatrix_graph::Graph;
use weavatrix_search_vector::{IndexConfig, VectorIndex};

/// First-party vector-index policy plus semantic candidate oversampling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorCandidateConfig {
    index: IndexConfig,
    candidate_pool_multiplier: usize,
}

impl VectorCandidateConfig {
    /// Creates the portable first-party vector-index policy for an embedding
    /// dimension.
    #[must_use]
    pub fn new(dimensions: usize) -> Self {
        Self {
            index: IndexConfig::new(dimensions),
            candidate_pool_multiplier: 2,
        }
    }

    /// Uses a caller-tuned first-party index policy.
    #[must_use]
    pub const fn from_index_config(index: IndexConfig) -> Self {
        Self {
            index,
            candidate_pool_multiplier: 2,
        }
    }

    /// Changes the number of vector candidates requested per semantic top-K
    /// slot.
    #[must_use]
    pub const fn with_candidate_pool_multiplier(
        mut self,
        candidate_pool_multiplier: usize,
    ) -> Self {
        self.candidate_pool_multiplier = candidate_pool_multiplier;
        self
    }

    /// First-party vector-index construction/query policy.
    #[must_use]
    pub const fn index_config(&self) -> &IndexConfig {
        &self.index
    }

    /// Candidate oversampling multiplier used before semantic reconciliation.
    #[must_use]
    pub const fn candidate_pool_multiplier(&self) -> usize {
        self.candidate_pool_multiplier
    }

    fn validate(&self) -> Result<()> {
        if self.candidate_pool_multiplier == 0 {
            return Err(SemanticError::ZeroCandidatePoolMultiplier);
        }
        self.index.validate()?;
        Ok(())
    }
}

/// Semantic linker backed by the first-party Weavatrix vector candidate index.
///
/// The vector index owns approximate candidate coverage only. This linker
/// still applies model thresholds, stable semantic top-K ordering, mutual or
/// union reconciliation, directional link policy, exact emitted-edge cosine
/// scoring, and graph provenance.
#[derive(Debug, Clone)]
pub struct VectorSemanticLinker {
    exact_semantics: SemanticLinker,
    candidates: VectorCandidateConfig,
}

impl VectorSemanticLinker {
    /// Validates semantic and vector-index policy and constructs a linker.
    ///
    /// # Errors
    ///
    /// Returns semantic or vector-index configuration failures.
    pub fn new(config: LinkConfig, candidates: VectorCandidateConfig) -> Result<Self> {
        candidates.validate()?;
        Ok(Self {
            exact_semantics: SemanticLinker::new(config)?,
            candidates,
        })
    }

    /// Active model-specific semantic configuration.
    #[must_use]
    pub const fn config(&self) -> &LinkConfig {
        self.exact_semantics.config()
    }

    /// Active first-party vector candidate policy.
    #[must_use]
    pub const fn candidate_config(&self) -> &VectorCandidateConfig {
        &self.candidates
    }

    /// Produces evidence-carrying semantic edges from first-party vector
    /// candidates.
    ///
    /// # Errors
    ///
    /// Returns graph/vector validation or vector-index build/query failures.
    pub fn link(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<SemanticLinkReport> {
        self.link_with_policy(graph, vectors, &AllowAllPolicy)
    }

    /// Produces evidence-carrying semantic edges under an explicit link policy.
    ///
    /// # Errors
    ///
    /// Returns graph/vector validation, policy coverage, or vector-index
    /// build/query failures.
    pub fn link_with_policy<P: LinkPolicy + ?Sized>(
        &self,
        graph: &Graph,
        vectors: &[SemanticVector],
        policy: &P,
    ) -> Result<SemanticLinkReport> {
        self.exact_semantics.validate_inputs(graph, vectors)?;
        policy.validate(graph, vectors)?;
        let ordered = ordered_vectors(vectors);
        if ordered.is_empty() {
            return self.exact_semantics.finish_link(
                &ordered,
                &[],
                0,
                CandidateBackend::WeavatrixVector,
                policy,
            );
        }

        let mut keyed = Vec::new();
        keyed
            .try_reserve_exact(ordered.len())
            .map_err(|_| SemanticError::AllocationFailed)?;
        for (key, vector) in ordered.iter().enumerate() {
            keyed.push((
                u64::try_from(key).map_err(|_| SemanticError::NumericOverflow)?,
                vector.values(),
            ));
        }
        let index = VectorIndex::build(self.candidates.index.clone(), &keyed)?;

        let queries = ordered
            .iter()
            .map(|vector| vector.values())
            .collect::<Vec<_>>();
        let candidate_count = self
            .config()
            .top_k()
            .saturating_mul(self.candidates.candidate_pool_multiplier)
            .saturating_add(1)
            .min(ordered.len());
        let hits = index.search_batch(&queries, candidate_count)?;
        let capacity = self.config().top_k().min(ordered.len().saturating_sub(1));
        let mut neighborhoods = Vec::new();
        neighborhoods
            .try_reserve_exact(ordered.len())
            .map_err(|_| SemanticError::AllocationFailed)?;

        for (source, found) in hits.into_iter().enumerate() {
            let mut candidates = Vec::with_capacity(capacity);
            for hit in found {
                let target = usize::try_from(hit.key).map_err(|_| {
                    SemanticError::CandidateKeyOutOfRange {
                        key: hit.key,
                        vector_count: ordered.len(),
                    }
                })?;
                if target >= ordered.len() {
                    return Err(SemanticError::CandidateKeyOutOfRange {
                        key: hit.key,
                        vector_count: ordered.len(),
                    });
                }
                if target == source {
                    continue;
                }
                if !policy.allows(ordered[source].node_id(), ordered[target].node_id()) {
                    continue;
                }
                let score = cosine(ordered[source], ordered[target]);
                if score >= self.config().min_similarity() {
                    retain_top_k(
                        &mut candidates,
                        Candidate { target, score },
                        self.config().top_k(),
                    );
                }
            }
            neighborhoods.push(candidates);
        }

        self.exact_semantics.finish_link(
            &ordered,
            &neighborhoods,
            0,
            CandidateBackend::WeavatrixVector,
            policy,
        )
    }

    /// Replaces only this crate's prior semantic edges and returns a new graph.
    ///
    /// # Errors
    ///
    /// Returns graph/vector validation or vector-index build/query failures.
    pub fn relink(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<Graph> {
        let report = self.link(graph, vectors)?;
        self.exact_semantics.replace_semantic_edges(graph, report)
    }

    /// Replaces this crate's prior edges using an explicit link policy.
    ///
    /// # Errors
    ///
    /// Returns graph/vector validation, policy coverage, vector-index, or
    /// graph-construction failures.
    pub fn relink_with_policy<P: LinkPolicy + ?Sized>(
        &self,
        graph: &Graph,
        vectors: &[SemanticVector],
        policy: &P,
    ) -> Result<Graph> {
        let report = self.link_with_policy(graph, vectors, policy)?;
        self.exact_semantics.replace_semantic_edges(graph, report)
    }
}
