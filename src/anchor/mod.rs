mod matcher;

use crate::{Result, SemanticError, SemanticVector};
use matcher::validate_candidate_text;
use weavatrix_graph::NodeId;

/// Configuration for matching extracted source text to link targets.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorConfig {
    model: String,
    min_similarity: f64,
    max_suggestions_per_link: usize,
}

impl AnchorConfig {
    /// Creates a model-specific anchor matching policy.
    #[must_use]
    pub fn new(
        model: impl Into<String>,
        min_similarity: f64,
        max_suggestions_per_link: usize,
    ) -> Self {
        Self {
            model: model.into(),
            min_similarity,
            max_suggestions_per_link,
        }
    }

    /// Embedding model expected on both links and text fragments.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Inclusive cosine threshold for a source text fragment.
    #[must_use]
    pub const fn min_similarity(&self) -> f64 {
        self.min_similarity
    }

    /// Maximum ranked anchor placements returned per directed link.
    #[must_use]
    pub const fn max_suggestions_per_link(&self) -> usize {
        self.max_suggestions_per_link
    }

    fn validate(&self) -> Result<()> {
        if self.model.is_empty() {
            return Err(SemanticError::EmptyAnchorModel);
        }
        if self.model.trim() != self.model {
            return Err(SemanticError::AnchorModelHasSurroundingWhitespace);
        }
        if !self.min_similarity.is_finite() || !(0.0..=1.0).contains(&self.min_similarity) {
            return Err(SemanticError::InvalidAnchorSimilarityThreshold);
        }
        if self.max_suggestions_per_link == 0 {
            return Err(SemanticError::ZeroAnchorSuggestions);
        }
        Ok(())
    }
}

/// Caller-extracted source text that could carry an internal link.
///
/// `locator` is an opaque stable location such as a DOM path or source span.
/// The vector should represent `context`; the exact `anchor_text` is preserved
/// for review or downstream HTML mutation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorCandidate {
    source: NodeId,
    locator: String,
    anchor_text: String,
    context: String,
    vector: SemanticVector,
}

impl AnchorCandidate {
    /// Creates and validates an extracted anchor candidate.
    ///
    /// # Errors
    ///
    /// Returns an error for empty metadata or an invalid semantic vector.
    pub fn new(
        source: NodeId,
        locator: impl Into<String>,
        anchor_text: impl Into<String>,
        context: impl Into<String>,
        values: Vec<f32>,
    ) -> Result<Self> {
        let locator = locator.into();
        let anchor_text = anchor_text.into();
        let context = context.into();
        if locator.is_empty() {
            return Err(SemanticError::EmptyAnchorLocator {
                source: source.to_string(),
            });
        }
        validate_candidate_text(&source, &locator, "locator", &locator)?;
        if anchor_text.is_empty() {
            return Err(SemanticError::EmptyAnchorText {
                source: source.to_string(),
                locator,
            });
        }
        validate_candidate_text(&source, &locator, "anchor_text", &anchor_text)?;
        if context.is_empty() {
            return Err(SemanticError::EmptyAnchorContext {
                source: source.to_string(),
                locator,
            });
        }
        validate_candidate_text(&source, &locator, "context", &context)?;
        if !context.contains(&anchor_text) {
            return Err(SemanticError::AnchorTextOutsideContext {
                source: source.to_string(),
                locator,
            });
        }
        let vector = SemanticVector::new(source.to_string(), values)?;
        Ok(Self {
            source,
            locator,
            anchor_text,
            context,
            vector,
        })
    }

    /// Source page containing the text.
    #[must_use]
    pub const fn source(&self) -> &NodeId {
        &self.source
    }

    /// Opaque caller-defined text location.
    #[must_use]
    pub fn locator(&self) -> &str {
        &self.locator
    }

    /// Exact existing source text proposed as the anchor.
    #[must_use]
    pub fn anchor_text(&self) -> &str {
        &self.anchor_text
    }

    /// Surrounding text used to compute the candidate vector.
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }
}

/// One ranked placement for a directed internal-link recommendation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorSuggestion {
    locator: String,
    anchor_text: String,
    context: String,
    similarity: f64,
}

impl AnchorSuggestion {
    /// Opaque source location.
    #[must_use]
    pub fn locator(&self) -> &str {
        &self.locator
    }

    /// Exact existing text proposed as anchor text.
    #[must_use]
    pub fn anchor_text(&self) -> &str {
        &self.anchor_text
    }

    /// Surrounding source text used for semantic matching.
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    /// Exact cosine similarity between source context and target page.
    #[must_use]
    pub const fn similarity(&self) -> f64 {
        self.similarity
    }
}

/// Ranked placements for one directed semantic link.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredLink {
    source: NodeId,
    target: NodeId,
    link_similarity: f64,
    suggestions: Vec<AnchorSuggestion>,
}

impl AnchoredLink {
    /// Source page.
    #[must_use]
    pub const fn source(&self) -> &NodeId {
        &self.source
    }

    /// Target page.
    #[must_use]
    pub const fn target(&self) -> &NodeId {
        &self.target
    }

    /// Page-level semantic similarity carried by the graph edge.
    #[must_use]
    pub const fn link_similarity(&self) -> f64 {
        self.link_similarity
    }

    /// Ranked, exact source-text placements.
    #[must_use]
    pub fn suggestions(&self) -> &[AnchorSuggestion] {
        &self.suggestions
    }
}

/// Evidence summary for one anchor-placement matching run.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorMatchReport {
    candidate_count: usize,
    comparisons: u64,
    links: Vec<AnchoredLink>,
}

impl AnchorMatchReport {
    /// Number of input text candidates.
    #[must_use]
    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    /// Exact fragment-to-target comparisons performed.
    #[must_use]
    pub const fn comparisons(&self) -> u64 {
        self.comparisons
    }

    /// Directed links, including links with no qualifying placement.
    #[must_use]
    pub fn links(&self) -> &[AnchoredLink] {
        &self.links
    }

    /// Number of links with at least one qualifying source placement.
    #[must_use]
    pub fn matched_link_count(&self) -> usize {
        self.links
            .iter()
            .filter(|link| !link.suggestions.is_empty())
            .count()
    }
}

/// Exact semantic matcher for source text placement and anchor review.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorMatcher {
    config: AnchorConfig,
}
