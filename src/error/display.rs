use super::SemanticError;
use std::fmt::{Display, Formatter};
use weavatrix_graph::GraphError;

impl Display for SemanticError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(result) = [
            format_vector_error(self, formatter),
            format_seo_error(self, formatter),
            format_anchor_error(self, formatter),
        ]
        .into_iter()
        .flatten()
        .next()
        {
            return result;
        }
        match self {
            #[cfg(feature = "vector-search")]
            Self::VectorSearch(error) => Display::fmt(error, formatter),
            Self::Graph(error) => Display::fmt(error, formatter),
            _ => formatter.write_str("unclassified semantic error"),
        }
    }
}

fn format_vector_error(
    error: &SemanticError,
    formatter: &mut Formatter<'_>,
) -> Option<std::fmt::Result> {
    Some(match error {
        SemanticError::EmptyModel => {
            formatter.write_str("embedding model identifier cannot be empty")
        }
        SemanticError::ModelHasSurroundingWhitespace => {
            formatter.write_str("embedding model identifier cannot have surrounding whitespace")
        }
        SemanticError::InvalidSimilarityThreshold => {
            formatter.write_str("minimum cosine similarity must be finite and within [0, 1]")
        }
        SemanticError::ZeroTopK => formatter.write_str("top_k must be greater than zero"),
        SemanticError::ZeroMaxVectors => {
            formatter.write_str("max_vectors must be greater than zero")
        }
        SemanticError::EmptyVector { node } => {
            write!(formatter, "semantic vector for {node} has no dimensions")
        }
        SemanticError::NonFiniteVectorValue { node, index } => write!(
            formatter,
            "semantic vector for {node} has a non-finite value at dimension {index}"
        ),
        SemanticError::ZeroVector { node } => {
            write!(formatter, "semantic vector for {node} has zero magnitude")
        }
        SemanticError::TooManyVectors { count, maximum } => write!(
            formatter,
            "semantic linker received {count} vectors; configured maximum is {maximum}"
        ),
        SemanticError::DuplicateNode { node } => {
            write!(
                formatter,
                "multiple semantic vectors target graph node {node}"
            )
        }
        SemanticError::MissingGraphNode { node } => {
            write!(
                formatter,
                "semantic vector targets missing graph node {node}"
            )
        }
        SemanticError::DimensionMismatch {
            node,
            expected,
            actual,
        } => write!(
            formatter,
            "semantic vector for {node} has {actual} dimensions; expected {expected}"
        ),
        SemanticError::NumericOverflow => {
            formatter.write_str("semantic-link metadata exceeds supported numeric range")
        }
        SemanticError::AllocationFailed => {
            formatter.write_str("semantic-link storage allocation failed")
        }
        SemanticError::ZeroCandidatePoolMultiplier => {
            formatter.write_str("vector candidate-pool multiplier must be greater than zero")
        }
        SemanticError::CandidateKeyOutOfRange { key, vector_count } => write!(
            formatter,
            "vector candidate key {key} is outside semantic input of {vector_count} vectors"
        ),
        _ => return None,
    })
}

fn format_seo_error(
    error: &SemanticError,
    formatter: &mut Formatter<'_>,
) -> Option<std::fmt::Result> {
    Some(match error {
        SemanticError::EmptySeoSite { node } => {
            write!(formatter, "SEO site identifier for {node} cannot be empty")
        }
        SemanticError::SeoSiteHasSurroundingWhitespace { node } => write!(
            formatter,
            "SEO site identifier for {node} cannot have surrounding whitespace"
        ),
        SemanticError::EmptySeoCanonical { node } => {
            write!(
                formatter,
                "SEO canonical identifier for {node} cannot be empty"
            )
        }
        SemanticError::SeoCanonicalHasSurroundingWhitespace { node } => write!(
            formatter,
            "SEO canonical identifier for {node} cannot have surrounding whitespace"
        ),
        SemanticError::EmptySeoLanguage { node } => {
            write!(
                formatter,
                "SEO language identifier for {node} cannot be empty"
            )
        }
        SemanticError::SeoLanguageHasSurroundingWhitespace { node } => write!(
            formatter,
            "SEO language identifier for {node} cannot have surrounding whitespace"
        ),
        SemanticError::DuplicateSeoProfile { node } => {
            write!(formatter, "multiple SEO profiles target graph node {node}")
        }
        SemanticError::MissingSeoProfile { node } => {
            write!(formatter, "semantic vector for {node} has no SEO profile")
        }
        SemanticError::SeoProfileMissingGraphNode { node } => {
            write!(formatter, "SEO profile targets missing graph node {node}")
        }
        _ => return None,
    })
}

fn format_anchor_error(
    error: &SemanticError,
    formatter: &mut Formatter<'_>,
) -> Option<std::fmt::Result> {
    Some(match error {
        SemanticError::EmptyAnchorModel => {
            formatter.write_str("anchor embedding model identifier cannot be empty")
        }
        SemanticError::AnchorModelHasSurroundingWhitespace => formatter
            .write_str("anchor embedding model identifier cannot have surrounding whitespace"),
        SemanticError::InvalidAnchorSimilarityThreshold => {
            formatter.write_str("minimum anchor cosine similarity must be finite and within [0, 1]")
        }
        SemanticError::ZeroAnchorSuggestions => {
            formatter.write_str("maximum anchor suggestions must be greater than zero")
        }
        SemanticError::EmptyAnchorLocator { source } => write!(
            formatter,
            "anchor candidate for {source} has an empty locator"
        ),
        SemanticError::EmptyAnchorText { source, locator } => write!(
            formatter,
            "anchor candidate {source} at {locator} has empty anchor text"
        ),
        SemanticError::EmptyAnchorContext { source, locator } => write!(
            formatter,
            "anchor candidate {source} at {locator} has empty context"
        ),
        SemanticError::AnchorTextOutsideContext { source, locator } => write!(
            formatter,
            "anchor candidate {source} at {locator} has anchor text outside its context"
        ),
        SemanticError::AnchorTextHasSurroundingWhitespace {
            source,
            locator,
            field,
        } => write!(
            formatter,
            "anchor candidate {source} at {locator} has surrounding whitespace in {field}"
        ),
        SemanticError::DuplicateAnchorCandidate { source, locator } => write!(
            formatter,
            "multiple anchor candidates target {source} at {locator}"
        ),
        SemanticError::MissingAnchorTargetVector { target } => write!(
            formatter,
            "semantic link target {target} has no vector for anchor matching"
        ),
        SemanticError::AnchorDimensionMismatch {
            source,
            locator,
            expected,
            actual,
        } => write!(
            formatter,
            "anchor candidate {source} at {locator} has {actual} dimensions; expected {expected}"
        ),
        SemanticError::AnchorModelMismatch { expected, actual } => write!(
            formatter,
            "semantic edge uses embedding model {actual}; anchor matcher expects {expected}"
        ),
        SemanticError::MissingSemanticEdgeAttribute { attribute } => write!(
            formatter,
            "semantic edge is missing required {attribute} attribute"
        ),
        _ => return None,
    })
}

impl std::error::Error for SemanticError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "vector-search")]
            Self::VectorSearch(error) => Some(error),
            Self::Graph(error) => Some(error),
            _ => None,
        }
    }
}

impl From<GraphError> for SemanticError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}

#[cfg(feature = "vector-search")]
impl From<weavatrix_search_vector::SearchError> for SemanticError {
    fn from(error: weavatrix_search_vector::SearchError) -> Self {
        Self::VectorSearch(error)
    }
}
