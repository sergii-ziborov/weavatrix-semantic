use super::SemanticError;
use std::fmt::{Display, Formatter};
use weavatrix_graph::GraphError;

impl Display for SemanticError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyModel => formatter.write_str("embedding model identifier cannot be empty"),
            Self::ModelHasSurroundingWhitespace => {
                formatter.write_str("embedding model identifier cannot have surrounding whitespace")
            }
            Self::InvalidSimilarityThreshold => {
                formatter.write_str("minimum cosine similarity must be finite and within [0, 1]")
            }
            Self::ZeroTopK => formatter.write_str("top_k must be greater than zero"),
            Self::ZeroMaxVectors => formatter.write_str("max_vectors must be greater than zero"),
            Self::EmptyVector { node } => {
                write!(formatter, "semantic vector for {node} has no dimensions")
            }
            Self::NonFiniteVectorValue { node, index } => write!(
                formatter,
                "semantic vector for {node} has a non-finite value at dimension {index}"
            ),
            Self::ZeroVector { node } => {
                write!(formatter, "semantic vector for {node} has zero magnitude")
            }
            Self::TooManyVectors { count, maximum } => write!(
                formatter,
                "semantic linker received {count} vectors; configured maximum is {maximum}"
            ),
            Self::DuplicateNode { node } => {
                write!(
                    formatter,
                    "multiple semantic vectors target graph node {node}"
                )
            }
            Self::MissingGraphNode { node } => {
                write!(
                    formatter,
                    "semantic vector targets missing graph node {node}"
                )
            }
            Self::DimensionMismatch {
                node,
                expected,
                actual,
            } => write!(
                formatter,
                "semantic vector for {node} has {actual} dimensions; expected {expected}"
            ),
            Self::NumericOverflow => {
                formatter.write_str("semantic-link metadata exceeds supported numeric range")
            }
            Self::AllocationFailed => {
                formatter.write_str("semantic-link storage allocation failed")
            }
            Self::ZeroCandidatePoolMultiplier => {
                formatter.write_str("vector candidate-pool multiplier must be greater than zero")
            }
            Self::CandidateKeyOutOfRange { key, vector_count } => write!(
                formatter,
                "vector candidate key {key} is outside semantic input of {vector_count} vectors"
            ),
            Self::EmptySeoSite { node } => {
                write!(formatter, "SEO site identifier for {node} cannot be empty")
            }
            Self::SeoSiteHasSurroundingWhitespace { node } => write!(
                formatter,
                "SEO site identifier for {node} cannot have surrounding whitespace"
            ),
            Self::EmptySeoCanonical { node } => {
                write!(
                    formatter,
                    "SEO canonical identifier for {node} cannot be empty"
                )
            }
            Self::SeoCanonicalHasSurroundingWhitespace { node } => write!(
                formatter,
                "SEO canonical identifier for {node} cannot have surrounding whitespace"
            ),
            Self::EmptySeoLanguage { node } => {
                write!(
                    formatter,
                    "SEO language identifier for {node} cannot be empty"
                )
            }
            Self::SeoLanguageHasSurroundingWhitespace { node } => write!(
                formatter,
                "SEO language identifier for {node} cannot have surrounding whitespace"
            ),
            Self::DuplicateSeoProfile { node } => {
                write!(formatter, "multiple SEO profiles target graph node {node}")
            }
            Self::MissingSeoProfile { node } => {
                write!(formatter, "semantic vector for {node} has no SEO profile")
            }
            Self::SeoProfileMissingGraphNode { node } => {
                write!(formatter, "SEO profile targets missing graph node {node}")
            }
            Self::EmptyAnchorModel => {
                formatter.write_str("anchor embedding model identifier cannot be empty")
            }
            Self::AnchorModelHasSurroundingWhitespace => formatter
                .write_str("anchor embedding model identifier cannot have surrounding whitespace"),
            Self::InvalidAnchorSimilarityThreshold => formatter
                .write_str("minimum anchor cosine similarity must be finite and within [0, 1]"),
            Self::ZeroAnchorSuggestions => {
                formatter.write_str("maximum anchor suggestions must be greater than zero")
            }
            Self::EmptyAnchorLocator { source } => write!(
                formatter,
                "anchor candidate for {source} has an empty locator"
            ),
            Self::EmptyAnchorText { source, locator } => write!(
                formatter,
                "anchor candidate {source} at {locator} has empty anchor text"
            ),
            Self::EmptyAnchorContext { source, locator } => write!(
                formatter,
                "anchor candidate {source} at {locator} has empty context"
            ),
            Self::AnchorTextOutsideContext { source, locator } => write!(
                formatter,
                "anchor candidate {source} at {locator} has anchor text outside its context"
            ),
            Self::AnchorTextHasSurroundingWhitespace {
                source,
                locator,
                field,
            } => write!(
                formatter,
                "anchor candidate {source} at {locator} has surrounding whitespace in {field}"
            ),
            Self::DuplicateAnchorCandidate { source, locator } => write!(
                formatter,
                "multiple anchor candidates target {source} at {locator}"
            ),
            Self::MissingAnchorTargetVector { target } => write!(
                formatter,
                "semantic link target {target} has no vector for anchor matching"
            ),
            Self::AnchorDimensionMismatch {
                source,
                locator,
                expected,
                actual,
            } => write!(
                formatter,
                "anchor candidate {source} at {locator} has {actual} dimensions; expected {expected}"
            ),
            Self::AnchorModelMismatch { expected, actual } => write!(
                formatter,
                "semantic edge uses embedding model {actual}; anchor matcher expects {expected}"
            ),
            Self::MissingSemanticEdgeAttribute { attribute } => write!(
                formatter,
                "semantic edge is missing required {attribute} attribute"
            ),
            #[cfg(feature = "vector-search")]
            Self::VectorSearch(error) => Display::fmt(error, formatter),
            Self::Graph(error) => Display::fmt(error, formatter),
        }
    }
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
