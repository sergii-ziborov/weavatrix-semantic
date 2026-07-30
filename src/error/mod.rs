mod display;
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
    /// More vectors were supplied than the caller-configured safety bound.
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
    /// Semantic candidate or report storage could not be reserved.
    AllocationFailed,
    /// The vector candidate-pool multiplier must be positive.
    ZeroCandidatePoolMultiplier,
    /// A candidate engine returned a key outside the current semantic input.
    CandidateKeyOutOfRange { key: u64, vector_count: usize },
    /// An SEO site identifier is empty.
    EmptySeoSite { node: String },
    /// An SEO site identifier has leading or trailing whitespace.
    SeoSiteHasSurroundingWhitespace { node: String },
    /// An SEO canonical identifier is empty.
    EmptySeoCanonical { node: String },
    /// An SEO canonical identifier has leading or trailing whitespace.
    SeoCanonicalHasSurroundingWhitespace { node: String },
    /// An SEO language identifier is empty.
    EmptySeoLanguage { node: String },
    /// An SEO language identifier has leading or trailing whitespace.
    SeoLanguageHasSurroundingWhitespace { node: String },
    /// More than one SEO profile targets the same graph node.
    DuplicateSeoProfile { node: String },
    /// A semantic vector has no corresponding SEO profile.
    MissingSeoProfile { node: String },
    /// An SEO profile targets a node absent from the graph.
    SeoProfileMissingGraphNode { node: String },
    /// The anchor matcher model identifier is empty.
    EmptyAnchorModel,
    /// The anchor matcher model identifier has surrounding whitespace.
    AnchorModelHasSurroundingWhitespace,
    /// The anchor similarity threshold is non-finite or outside `[0, 1]`.
    InvalidAnchorSimilarityThreshold,
    /// At least one anchor suggestion per link must be requested.
    ZeroAnchorSuggestions,
    /// A candidate anchor locator is empty.
    EmptyAnchorLocator { source: String },
    /// Candidate anchor text is empty.
    EmptyAnchorText { source: String, locator: String },
    /// Candidate anchor context is empty.
    EmptyAnchorContext { source: String, locator: String },
    /// Candidate anchor text is not present in its supplied source context.
    AnchorTextOutsideContext { source: String, locator: String },
    /// Candidate anchor text metadata has surrounding whitespace.
    AnchorTextHasSurroundingWhitespace {
        source: String,
        locator: String,
        field: &'static str,
    },
    /// More than one candidate uses the same source and locator.
    DuplicateAnchorCandidate { source: String, locator: String },
    /// A semantic link target has no page vector for anchor matching.
    MissingAnchorTargetVector { target: String },
    /// A candidate anchor vector does not match the target-vector dimension.
    AnchorDimensionMismatch {
        source: String,
        locator: String,
        expected: usize,
        actual: usize,
    },
    /// A semantic edge was produced by another embedding model.
    AnchorModelMismatch { expected: String, actual: String },
    /// A semantic edge is missing an attribute required for anchor matching.
    MissingSemanticEdgeAttribute { attribute: &'static str },
    /// The first-party vector candidate engine rejected a build or query.
    #[cfg(feature = "vector-search")]
    VectorSearch(weavatrix_search_vector::SearchError),
    /// The underlying graph rejected a kind, identifier, provenance, or edge.
    Graph(GraphError),
}
