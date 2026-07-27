use super::{
    AnchorCandidate, AnchorConfig, AnchorMatchReport, AnchorMatcher, AnchorSuggestion, AnchoredLink,
};
use crate::{Result, SemanticError, SemanticLinkReport, SemanticVector};
use std::collections::{BTreeMap, BTreeSet};
use weavatrix_graph::{AttributeValue, NodeId};

impl AnchorMatcher {
    /// Validates configuration and creates an anchor matcher.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid model, threshold, or suggestion count.
    pub fn new(config: AnchorConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    #[must_use]
    pub const fn config(&self) -> &AnchorConfig {
        &self.config
    }

    /// Matches caller-extracted source text to directed semantic link targets.
    ///
    /// # Errors
    ///
    /// Returns duplicate, missing-vector, dimension, model, or numeric errors.
    pub fn match_links(
        &self,
        report: &SemanticLinkReport,
        page_vectors: &[SemanticVector],
        candidates: &[AnchorCandidate],
    ) -> Result<AnchorMatchReport> {
        let mut indexed_vectors = BTreeMap::new();
        for vector in page_vectors {
            if indexed_vectors.insert(vector.node_id(), vector).is_some() {
                return Err(SemanticError::DuplicateNode {
                    node: vector.node_id().to_string(),
                });
            }
        }
        let by_source = index_candidates(candidates)?;
        let mut comparisons = 0_u64;
        let mut links = Vec::with_capacity(report.edge_count());
        for edge in report.edges() {
            validate_edge_model(edge, self.config.model())?;
            let target = indexed_vectors.get(&edge.target).ok_or_else(|| {
                SemanticError::MissingAnchorTargetVector {
                    target: edge.target.to_string(),
                }
            })?;
            let mut suggestions = Vec::new();
            for candidate in by_source
                .get(&edge.source)
                .into_iter()
                .flat_map(|values| values.iter())
            {
                validate_dimension(candidate, target)?;
                comparisons = comparisons
                    .checked_add(1)
                    .ok_or(SemanticError::NumericOverflow)?;
                let similarity = cosine(&candidate.vector, target);
                if similarity >= self.config.min_similarity() {
                    suggestions.push(AnchorSuggestion {
                        locator: candidate.locator.clone(),
                        anchor_text: candidate.anchor_text.clone(),
                        context: candidate.context.clone(),
                        similarity,
                    });
                }
            }
            suggestions.sort_unstable_by(compare_suggestions);
            suggestions.truncate(self.config.max_suggestions_per_link());
            links.push(AnchoredLink {
                source: edge.source.clone(),
                target: edge.target.clone(),
                link_similarity: edge_similarity(edge)?,
                suggestions,
            });
        }
        Ok(AnchorMatchReport {
            candidate_count: candidates.len(),
            comparisons,
            links,
        })
    }
}

fn index_candidates(
    candidates: &[AnchorCandidate],
) -> Result<BTreeMap<&NodeId, Vec<&AnchorCandidate>>> {
    let mut by_source = BTreeMap::<&NodeId, Vec<&AnchorCandidate>>::new();
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        let identity = (candidate.source(), candidate.locator());
        if !seen.insert(identity) {
            return Err(SemanticError::DuplicateAnchorCandidate {
                source: candidate.source.to_string(),
                locator: candidate.locator.clone(),
            });
        }
        by_source
            .entry(candidate.source())
            .or_default()
            .push(candidate);
    }
    Ok(by_source)
}

fn validate_dimension(candidate: &AnchorCandidate, target: &SemanticVector) -> Result<()> {
    if candidate.vector.dimension() != target.dimension() {
        return Err(SemanticError::AnchorDimensionMismatch {
            source: candidate.source.to_string(),
            locator: candidate.locator.clone(),
            expected: target.dimension(),
            actual: candidate.vector.dimension(),
        });
    }
    Ok(())
}

pub(super) fn validate_candidate_text(
    source: &NodeId,
    locator: &str,
    field: &'static str,
    value: &str,
) -> Result<()> {
    if value.trim() != value {
        return Err(SemanticError::AnchorTextHasSurroundingWhitespace {
            source: source.to_string(),
            locator: locator.to_owned(),
            field,
        });
    }
    Ok(())
}

fn validate_edge_model(edge: &weavatrix_graph::Edge, expected: &str) -> Result<()> {
    let Some(AttributeValue::String(actual)) = edge.attributes.get("model") else {
        return Err(SemanticError::MissingSemanticEdgeAttribute { attribute: "model" });
    };
    if actual != expected {
        return Err(SemanticError::AnchorModelMismatch {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn edge_similarity(edge: &weavatrix_graph::Edge) -> Result<f64> {
    match edge.attributes.get("similarity") {
        Some(AttributeValue::Float(score)) => Ok(score.get()),
        _ => Err(SemanticError::MissingSemanticEdgeAttribute {
            attribute: "similarity",
        }),
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

fn compare_suggestions(left: &AnchorSuggestion, right: &AnchorSuggestion) -> std::cmp::Ordering {
    right
        .similarity
        .total_cmp(&left.similarity)
        .then_with(|| left.locator.cmp(&right.locator))
        .then_with(|| left.anchor_text.cmp(&right.anchor_text))
}
