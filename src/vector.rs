use crate::{Result, SemanticError};
use weavatrix_graph::NodeId;

/// One graph node represented in a named embedding space.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticVector {
    node_id: NodeId,
    values: Vec<f32>,
    norm: f64,
}

impl SemanticVector {
    /// Creates and validates a finite, non-zero embedding vector.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty node identifier, empty vector, non-finite
    /// component, or zero-magnitude vector.
    pub fn new(node_id: impl Into<String>, values: Vec<f32>) -> Result<Self> {
        let node_id = NodeId::new(node_id)?;
        if values.is_empty() {
            return Err(SemanticError::EmptyVector {
                node: node_id.to_string(),
            });
        }

        let mut squared_norm = 0.0_f64;
        for (index, value) in values.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(SemanticError::NonFiniteVectorValue {
                    node: node_id.to_string(),
                    index,
                });
            }
            squared_norm += f64::from(value) * f64::from(value);
        }
        if squared_norm == 0.0 {
            return Err(SemanticError::ZeroVector {
                node: node_id.to_string(),
            });
        }

        Ok(Self {
            node_id,
            values,
            norm: squared_norm.sqrt(),
        })
    }

    /// Graph node represented by this vector.
    #[must_use]
    pub const fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Embedding components.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Number of embedding dimensions.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.values.len()
    }

    pub(crate) const fn norm(&self) -> f64 {
        self.norm
    }
}
