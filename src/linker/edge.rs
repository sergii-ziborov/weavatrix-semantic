use super::{CandidateBackend, SEMANTIC_EXTRACTOR, SemanticLinker};
use crate::{LinkPolicy, Result, SelectionMode, SemanticError, SemanticVector};
use std::collections::{BTreeMap, BTreeSet};
use weavatrix_graph::{AttributeValue, Edge, EvidenceKind, FiniteF64, NodeId, Provenance};

impl SemanticLinker {
    pub(super) fn select_pairs<P: LinkPolicy + ?Sized>(
        &self,
        ordered: &[&SemanticVector],
        ranks: &[BTreeMap<usize, usize>],
        policy: &P,
    ) -> BTreeSet<(usize, usize)> {
        let mut directed = BTreeSet::new();
        for (source, selected) in ranks.iter().enumerate() {
            for &target in selected.keys() {
                match self.config.selection() {
                    SelectionMode::Directed => {
                        directed.insert((source, target));
                    }
                    SelectionMode::Mutual if ranks[target].contains_key(&source) => {
                        directed.insert((source, target));
                    }
                    SelectionMode::Union => {
                        directed.insert((source, target));
                        if policy.allows(ordered[target].node_id(), ordered[source].node_id()) {
                            directed.insert((target, source));
                        }
                    }
                    SelectionMode::Mutual => {}
                }
            }
        }
        directed
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn make_edge<P: LinkPolicy + ?Sized>(
        &self,
        source: &NodeId,
        target: &NodeId,
        score: f64,
        dimension: usize,
        source_rank: Option<usize>,
        target_rank: Option<usize>,
        backend: CandidateBackend,
        policy: &P,
    ) -> Result<Edge> {
        let dimension = u64::try_from(dimension).map_err(|_| SemanticError::NumericOverflow)?;
        let provenance = Provenance::new(
            SEMANTIC_EXTRACTOR,
            EvidenceKind::Inferred,
            self.config.confidence(),
        )?
        .with_detail(format!(
            "model={}; metric=cosine; candidate_backend={}",
            self.config.model(),
            backend.as_str()
        ));
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
        .with_attribute("candidate_backend", backend.as_str())
        .with_attribute("candidate_exact", backend.is_exact())
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
        policy.annotate_edge(edge, source, target)
    }
}
