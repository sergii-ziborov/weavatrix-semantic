use crate::{Result, SemanticError};
use weavatrix_graph::Confidence;

/// Determines how directed top-K choices become symmetric semantic pairs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// Keep only pairs where both endpoints selected each other.
    #[default]
    Mutual,
    /// Keep pairs where either endpoint selected the other.
    Union,
}

impl SelectionMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Mutual => "mutual",
            Self::Union => "union",
        }
    }
}

/// Configuration for exact cosine semantic linking.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkConfig {
    model: String,
    min_similarity: f64,
    top_k: usize,
    selection: SelectionMode,
    confidence: Confidence,
    max_vectors: usize,
}

impl LinkConfig {
    /// Creates configuration with explicit model-specific threshold and top-K.
    #[must_use]
    pub fn new(model: impl Into<String>, min_similarity: f64, top_k: usize) -> Self {
        Self {
            model: model.into(),
            min_similarity,
            top_k,
            selection: SelectionMode::Mutual,
            confidence: Confidence::Low,
            max_vectors: 5_000,
        }
    }

    /// Changes pair selection semantics.
    #[must_use]
    pub const fn with_selection(mut self, selection: SelectionMode) -> Self {
        self.selection = selection;
        self
    }

    /// Sets the confidence attached to inferred graph evidence.
    #[must_use]
    pub const fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    /// Sets a hard bound for the exact O(n²) implementation.
    #[must_use]
    pub const fn with_max_vectors(mut self, max_vectors: usize) -> Self {
        self.max_vectors = max_vectors;
        self
    }

    /// Embedding model identifier stored on every emitted edge.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Inclusive cosine threshold in the range `[0, 1]`.
    #[must_use]
    pub const fn min_similarity(&self) -> f64 {
        self.min_similarity
    }

    /// Maximum selected neighbors per vector before pair reconciliation.
    #[must_use]
    pub const fn top_k(&self) -> usize {
        self.top_k
    }

    /// Pair selection mode.
    #[must_use]
    pub const fn selection(&self) -> SelectionMode {
        self.selection
    }

    /// Confidence attached to all inferred edges.
    #[must_use]
    pub const fn confidence(&self) -> Confidence {
        self.confidence
    }

    /// Maximum accepted vector count.
    #[must_use]
    pub const fn max_vectors(&self) -> usize {
        self.max_vectors
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.model.is_empty() {
            return Err(SemanticError::EmptyModel);
        }
        if self.model.trim() != self.model {
            return Err(SemanticError::ModelHasSurroundingWhitespace);
        }
        if !self.min_similarity.is_finite() || !(0.0..=1.0).contains(&self.min_similarity) {
            return Err(SemanticError::InvalidSimilarityThreshold);
        }
        if self.top_k == 0 {
            return Err(SemanticError::ZeroTopK);
        }
        if self.max_vectors == 0 {
            return Err(SemanticError::ZeroMaxVectors);
        }
        Ok(())
    }
}
