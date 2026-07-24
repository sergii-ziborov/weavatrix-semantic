#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod config;
mod error;
mod linker;
mod vector;

pub use config::{LinkConfig, SelectionMode};
pub use error::{Result, SemanticError};
pub use linker::{SEMANTIC_EDGE_KIND, SEMANTIC_EXTRACTOR, SemanticLinkReport, SemanticLinker};
pub use vector::SemanticVector;
