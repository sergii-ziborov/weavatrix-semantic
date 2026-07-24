use std::fmt::{Display, Formatter};
use weavatrix_graph::GraphError;

/// Result type returned by semantic-linking operations.
pub type Result<T> = std::result::Result<T, SemanticError>;

/// Validation and graph-integration failures.
#[derive(Debug)]
#[non_exhaustive]
pub enum SemanticError {
    /// The model identifier is empty.
    EmptyModel,
    /// The model identifier has leading or trailing whitespace.
    ModelHasSurroundingWhitespace,
    /// The cosine threshold is non-finite or outside `[0, 1]`.
    InvalidSimilarityThreshold,
    /// Top-K must be positive.
    ZeroTopK,
    /// The configured vector limit must be positive.
    ZeroMaxVectors,
    /// A vector has no dimensions.
    EmptyVector { node: String },
    /// A vector contains NaN or infinity.
    NonFiniteVectorValue { node: String, index: usize },
    /// A vector has no direction and cannot be compared with cosine similarity.
    ZeroVector { node: String },
    /// More vectors were supplied than the exact implementation permits.
    TooManyVectors { count: usize, maximum: usize },
    /// Two vectors target the same graph node.
    DuplicateNode { node: String },
    /// A vector targets a node absent from the input graph.
    MissingGraphNode { node: String },
    /// Vectors from the same model have inconsistent dimensions.
    DimensionMismatch {
        node: String,
        expected: usize,
        actual: usize,
    },
    /// A platform-sized counter could not be represented in graph metadata.
    NumericOverflow,
    /// The underlying graph rejected a kind, identifier, provenance, or edge.
    Graph(GraphError),
}

impl Display for SemanticError {
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
            Self::NonFiniteVectorValue { node, index } => {
                write!(
                    formatter,
                    "semantic vector for {node} has a non-finite value at dimension {index}"
                )
            }
            Self::ZeroVector { node } => {
                write!(formatter, "semantic vector for {node} has zero magnitude")
            }
            Self::TooManyVectors { count, maximum } => {
                write!(
                    formatter,
                    "exact semantic linker received {count} vectors; configured maximum is {maximum}"
                )
            }
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
            } => {
                write!(
                    formatter,
                    "semantic vector for {node} has {actual} dimensions; expected {expected}"
                )
            }
            Self::NumericOverflow => {
                formatter.write_str("semantic-link metadata exceeds supported numeric range")
            }
            Self::Graph(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for SemanticError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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
