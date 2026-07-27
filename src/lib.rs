#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod anchor;
mod config;
mod error;
mod linker;
mod policy;
mod vector;
#[cfg(feature = "vector-search")]
mod vector_linker;

pub use anchor::{
    AnchorCandidate, AnchorConfig, AnchorMatchReport, AnchorMatcher, AnchorSuggestion, AnchoredLink,
};
pub use config::{LinkConfig, SelectionMode};
pub use error::{Result, SemanticError};
pub use linker::{
    CandidateBackend, SEMANTIC_EDGE_KIND, SEMANTIC_EXTRACTOR, SemanticLinkReport, SemanticLinker,
};
pub use policy::{AllowAllPolicy, LinkPolicy, SeoLinkPolicy, SeoPage};
pub use vector::SemanticVector;
#[cfg(feature = "vector-search")]
pub use vector_linker::{VectorCandidateConfig, VectorSemanticLinker};
#[cfg(feature = "vector-search")]
pub use weavatrix_search_vector::{
    DistanceMetric as VectorDistanceMetric, IndexConfig as VectorIndexConfig,
};
