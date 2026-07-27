use super::math::{cosine, ordered_vectors, retain_top_k};
use super::{
    Candidate, CandidateBackend, SEMANTIC_EDGE_KIND, SEMANTIC_EXTRACTOR, SemanticLinkReport,
    SemanticLinker,
};
use crate::{AllowAllPolicy, LinkConfig, LinkPolicy, Result, SemanticError, SemanticVector};
use std::collections::{BTreeMap, BTreeSet};
use weavatrix_graph::{EdgeKind, Graph, GraphBuilder};

impl SemanticLinker {
    /// Validates configuration and constructs a linker.
    ///
    /// # Errors
    ///
    /// Returns invalid model, threshold, top-K, vector bound, or edge kind.
    pub fn new(config: LinkConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            edge_kind: EdgeKind::custom(SEMANTIC_EDGE_KIND)?,
        })
    }

    #[must_use]
    pub const fn config(&self) -> &LinkConfig {
        &self.config
    }

    /// Produces deterministic semantic edges without mutating the graph.
    ///
    /// # Errors
    ///
    /// Returns vector or graph validation failures.
    pub fn link(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<SemanticLinkReport> {
        self.link_with_policy(graph, vectors, &AllowAllPolicy)
    }

    /// Produces deterministic semantic edges under an explicit policy.
    ///
    /// # Errors
    ///
    /// Returns input validation or policy coverage failures.
    pub fn link_with_policy<P: LinkPolicy + ?Sized>(
        &self,
        graph: &Graph,
        vectors: &[SemanticVector],
        policy: &P,
    ) -> Result<SemanticLinkReport> {
        self.validate_inputs(graph, vectors)?;
        policy.validate(graph, vectors)?;
        let ordered = ordered_vectors(vectors);
        let capacity = self.config.top_k().min(ordered.len().saturating_sub(1));
        let mut neighborhoods = (0..ordered.len())
            .map(|_| Vec::<Candidate>::with_capacity(capacity))
            .collect::<Vec<_>>();
        let mut comparisons = 0_u64;
        for source in 0..ordered.len() {
            for target in (source + 1)..ordered.len() {
                let forward = policy.allows(ordered[source].node_id(), ordered[target].node_id());
                let reverse = policy.allows(ordered[target].node_id(), ordered[source].node_id());
                if !forward && !reverse {
                    continue;
                }
                comparisons = comparisons
                    .checked_add(1)
                    .ok_or(SemanticError::NumericOverflow)?;
                let score = cosine(ordered[source], ordered[target]);
                if score < self.config.min_similarity() {
                    continue;
                }
                if forward {
                    retain_top_k(
                        &mut neighborhoods[source],
                        Candidate { target, score },
                        self.config.top_k(),
                    );
                }
                if reverse {
                    retain_top_k(
                        &mut neighborhoods[target],
                        Candidate {
                            target: source,
                            score,
                        },
                        self.config.top_k(),
                    );
                }
            }
        }
        self.finish_link(
            &ordered,
            &neighborhoods,
            comparisons,
            CandidateBackend::Exact,
            policy,
        )
    }

    pub(crate) fn finish_link<P: LinkPolicy + ?Sized>(
        &self,
        ordered: &[&SemanticVector],
        neighborhoods: &[Vec<Candidate>],
        comparisons: u64,
        candidate_backend: CandidateBackend,
        policy: &P,
    ) -> Result<SemanticLinkReport> {
        if ordered.is_empty() {
            return Ok(SemanticLinkReport {
                vector_count: 0,
                dimension: 0,
                comparisons,
                pair_count: 0,
                candidate_backend,
                policy_id: policy.id().to_owned(),
                edges: Vec::new(),
            });
        }
        let ranks = neighborhoods
            .iter()
            .map(|candidates| {
                candidates
                    .iter()
                    .enumerate()
                    .map(|(rank, candidate)| (candidate.target, rank + 1))
                    .collect::<BTreeMap<_, _>>()
            })
            .collect::<Vec<_>>();
        let directed = self.select_pairs(ordered, &ranks, policy);
        let dimension = ordered[0].dimension();
        let mut edges = Vec::with_capacity(directed.len());
        let mut unordered_pairs = BTreeSet::new();
        for (source, target) in directed {
            let score = cosine(ordered[source], ordered[target]);
            if score < self.config.min_similarity() {
                continue;
            }
            edges.push(self.make_edge(
                ordered[source].node_id(),
                ordered[target].node_id(),
                score,
                dimension,
                ranks[source].get(&target).copied(),
                ranks[target].get(&source).copied(),
                candidate_backend,
                policy,
            )?);
            unordered_pairs.insert((source.min(target), source.max(target)));
        }
        Ok(SemanticLinkReport {
            vector_count: ordered.len(),
            dimension,
            comparisons,
            pair_count: unordered_pairs.len(),
            candidate_backend,
            policy_id: policy.id().to_owned(),
            edges,
        })
    }

    /// Replaces previous semantic edges and returns a new graph.
    ///
    /// # Errors
    ///
    /// Returns input validation or graph-construction failures.
    pub fn relink(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<Graph> {
        let report = self.link(graph, vectors)?;
        self.replace_semantic_edges(graph, report)
    }

    /// Replaces previous semantic edges under an explicit policy.
    ///
    /// # Errors
    ///
    /// Returns input validation, policy, or graph-construction failures.
    pub fn relink_with_policy<P: LinkPolicy + ?Sized>(
        &self,
        graph: &Graph,
        vectors: &[SemanticVector],
        policy: &P,
    ) -> Result<Graph> {
        let report = self.link_with_policy(graph, vectors, policy)?;
        self.replace_semantic_edges(graph, report)
    }

    pub(crate) fn replace_semantic_edges(
        &self,
        graph: &Graph,
        report: SemanticLinkReport,
    ) -> Result<Graph> {
        let mut builder = GraphBuilder::with_capacity(
            graph.node_count(),
            graph.edge_count().saturating_add(report.edges().len()),
        );
        for node in graph.nodes() {
            builder.add_node(node.clone())?;
        }
        for edge in graph.edges() {
            if edge.kind != self.edge_kind || edge.provenance.extractor != SEMANTIC_EXTRACTOR {
                builder.add_edge(edge.clone())?;
            }
        }
        for edge in report.into_edges() {
            builder.add_edge(edge)?;
        }
        Ok(builder.build()?)
    }

    pub(crate) fn validate_inputs(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<()> {
        if vectors.len() > self.config.max_vectors() {
            return Err(SemanticError::TooManyVectors {
                count: vectors.len(),
                maximum: self.config.max_vectors(),
            });
        }
        let expected = vectors.first().map_or(0, SemanticVector::dimension);
        let mut seen = BTreeSet::new();
        for vector in vectors {
            let node = vector.node_id().as_str();
            if graph.node(node).is_none() {
                return Err(SemanticError::MissingGraphNode {
                    node: node.to_owned(),
                });
            }
            if !seen.insert(vector.node_id()) {
                return Err(SemanticError::DuplicateNode {
                    node: node.to_owned(),
                });
            }
            if vector.dimension() != expected {
                return Err(SemanticError::DimensionMismatch {
                    node: node.to_owned(),
                    expected,
                    actual: vector.dimension(),
                });
            }
        }
        Ok(())
    }
}
