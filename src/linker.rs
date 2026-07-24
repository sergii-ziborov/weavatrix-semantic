use crate::{LinkConfig, Result, SelectionMode, SemanticError, SemanticVector};
use std::collections::{BTreeMap, BTreeSet};
use weavatrix_graph::{
    AttributeValue, Edge, EdgeKind, EvidenceKind, FiniteF64, Graph, GraphBuilder, NodeId,
    Provenance,
};

/// Custom graph edge kind emitted by this crate.
pub const SEMANTIC_EDGE_KIND: &str = "semantic_similarity";

/// Stable provenance extractor identity used for idempotent relinking.
pub const SEMANTIC_EXTRACTOR: &str = "weavatrix-semantic";

#[derive(Debug, Clone, Copy)]
struct Candidate {
    target: usize,
    score: f64,
}

/// Deterministic summary of one semantic-linking run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticLinkReport {
    vector_count: usize,
    dimension: usize,
    comparisons: u64,
    pair_count: usize,
    edges: Vec<Edge>,
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

    /// Exact pairwise cosine comparisons performed.
    #[must_use]
    pub const fn comparisons(&self) -> u64 {
        self.comparisons
    }

    /// Number of unordered semantic pairs retained.
    #[must_use]
    pub const fn pair_count(&self) -> usize {
        self.pair_count
    }

    /// Directed graph edges; every retained pair contributes two.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Consumes the report and returns its graph edges.
    #[must_use]
    pub fn into_edges(self) -> Vec<Edge> {
        self.edges
    }
}

/// Exact cosine top-K semantic linker for existing graph nodes.
#[derive(Debug, Clone)]
pub struct SemanticLinker {
    config: LinkConfig,
    edge_kind: EdgeKind,
}

impl SemanticLinker {
    /// Validates configuration and constructs a linker.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty model, invalid threshold, zero top-K,
    /// zero vector bound, or invalid semantic edge kind.
    pub fn new(config: LinkConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            edge_kind: EdgeKind::custom(SEMANTIC_EDGE_KIND)?,
        })
    }

    /// Active linker configuration.
    #[must_use]
    pub const fn config(&self) -> &LinkConfig {
        &self.config
    }

    /// Produces deterministic semantic edges without mutating the input graph.
    ///
    /// # Errors
    ///
    /// Returns an error when vectors exceed the configured bound, target
    /// missing or duplicate graph nodes, or use inconsistent dimensions.
    pub fn link(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<SemanticLinkReport> {
        self.validate_inputs(graph, vectors)?;
        if vectors.is_empty() {
            return Ok(SemanticLinkReport {
                vector_count: 0,
                dimension: 0,
                comparisons: 0,
                pair_count: 0,
                edges: Vec::new(),
            });
        }

        let mut ordered = vectors.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by(|left, right| left.node_id().cmp(right.node_id()));

        let mut neighborhoods = vec![Vec::<Candidate>::new(); ordered.len()];
        let mut comparisons = 0_u64;
        for source in 0..ordered.len() {
            for target in (source + 1)..ordered.len() {
                comparisons = comparisons
                    .checked_add(1)
                    .ok_or(SemanticError::NumericOverflow)?;
                let score = cosine(ordered[source], ordered[target]);
                if score >= self.config.min_similarity() {
                    neighborhoods[source].push(Candidate { target, score });
                    neighborhoods[target].push(Candidate {
                        target: source,
                        score,
                    });
                }
            }
        }

        for candidates in &mut neighborhoods {
            candidates.sort_unstable_by(|left, right| {
                right.score.total_cmp(&left.score).then_with(|| {
                    ordered[left.target]
                        .node_id()
                        .cmp(ordered[right.target].node_id())
                })
            });
            candidates.truncate(self.config.top_k());
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

        let mut pairs = BTreeSet::new();
        for (source, selected) in ranks.iter().enumerate() {
            for &target in selected.keys() {
                let keep = self.config.selection() == SelectionMode::Union
                    || ranks[target].contains_key(&source);
                if keep {
                    pairs.insert((source.min(target), source.max(target)));
                }
            }
        }

        let dimension = ordered[0].dimension();
        let mut edges = Vec::with_capacity(pairs.len().saturating_mul(2));
        for (left, right) in pairs {
            let score = cosine(ordered[left], ordered[right]);
            edges.push(self.make_edge(
                ordered[left].node_id(),
                ordered[right].node_id(),
                score,
                dimension,
                ranks[left].get(&right).copied(),
                ranks[right].get(&left).copied(),
            )?);
            edges.push(self.make_edge(
                ordered[right].node_id(),
                ordered[left].node_id(),
                score,
                dimension,
                ranks[right].get(&left).copied(),
                ranks[left].get(&right).copied(),
            )?);
        }

        Ok(SemanticLinkReport {
            vector_count: ordered.len(),
            dimension,
            comparisons,
            pair_count: edges.len() / 2,
            edges,
        })
    }

    /// Replaces this linker's previous semantic edges and returns a new graph.
    ///
    /// Nodes and evidence emitted by every other extractor are preserved.
    ///
    /// # Errors
    ///
    /// Returns input validation or graph-construction errors.
    pub fn relink(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<Graph> {
        let report = self.link(graph, vectors)?;
        let mut builder = GraphBuilder::with_capacity(
            graph.node_count(),
            graph.edge_count().saturating_add(report.edges().len()),
        );
        for node in graph.nodes() {
            builder.add_node(node.clone())?;
        }
        for edge in graph.edges() {
            if edge.kind == self.edge_kind && edge.provenance.extractor == SEMANTIC_EXTRACTOR {
                continue;
            }
            builder.add_edge(edge.clone())?;
        }
        for edge in report.into_edges() {
            builder.add_edge(edge)?;
        }
        Ok(builder.build()?)
    }

    fn validate_inputs(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<()> {
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

    #[allow(clippy::too_many_arguments)]
    fn make_edge(
        &self,
        source: &NodeId,
        target: &NodeId,
        score: f64,
        dimension: usize,
        source_rank: Option<usize>,
        target_rank: Option<usize>,
    ) -> Result<Edge> {
        let dimension = u64::try_from(dimension).map_err(|_| SemanticError::NumericOverflow)?;
        let provenance = Provenance::new(
            SEMANTIC_EXTRACTOR,
            EvidenceKind::Inferred,
            self.config.confidence(),
        )?
        .with_detail(format!("model={}; metric=cosine", self.config.model()));

        let mut edge = Edge::new(
            source.clone(),
            target.clone(),
            self.edge_kind.clone(),
            provenance,
        )
        .with_attribute(
            "similarity",
            AttributeValue::Float(
                FiniteF64::new(score).map_err(|_| SemanticError::NumericOverflow)?,
            ),
        )
        .with_attribute("metric", "cosine")
        .with_attribute("model", self.config.model())
        .with_attribute("dimensions", dimension)
        .with_attribute("selection", self.config.selection().as_str())
        .with_attribute("selected_by_source", source_rank.is_some())
        .with_attribute("selected_by_target", target_rank.is_some())
        .with_attribute("linker_version", env!("CARGO_PKG_VERSION"));

        if let Some(rank) = source_rank {
            edge = edge.with_attribute(
                "source_rank",
                u64::try_from(rank).map_err(|_| SemanticError::NumericOverflow)?,
            );
        }
        if let Some(rank) = target_rank {
            edge = edge.with_attribute(
                "target_rank",
                u64::try_from(rank).map_err(|_| SemanticError::NumericOverflow)?,
            );
        }
        Ok(edge)
    }
}

fn cosine(left: &SemanticVector, right: &SemanticVector) -> f64 {
    let dot = left
        .values()
        .iter()
        .zip(right.values())
        .map(|(&left, &right)| f64::from(left) * f64::from(right))
        .sum::<f64>();
    (dot / (left.norm() * right.norm())).clamp(-1.0, 1.0)
}
