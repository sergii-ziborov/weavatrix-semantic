use super::{LinkPolicy, SeoPage};
use crate::{Result, SemanticError, SemanticVector};
use std::collections::BTreeMap;
use weavatrix_graph::{Edge, Graph, NodeId};

/// Directional internal-link policy for SEO recommendation graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeoLinkPolicy {
    pages: BTreeMap<NodeId, SeoPage>,
    allow_cross_language: bool,
}

impl SeoLinkPolicy {
    /// Builds an SEO policy and rejects duplicate page profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when more than one profile targets the same node.
    pub fn new(pages: impl IntoIterator<Item = SeoPage>) -> Result<Self> {
        let mut indexed = BTreeMap::new();
        for page in pages {
            let node = page.node_id.clone();
            if indexed.insert(node.clone(), page).is_some() {
                return Err(SemanticError::DuplicateSeoProfile {
                    node: node.to_string(),
                });
            }
        }
        Ok(Self {
            pages: indexed,
            allow_cross_language: false,
        })
    }

    #[must_use]
    pub const fn with_cross_language(mut self, allow: bool) -> Self {
        self.allow_cross_language = allow;
        self
    }

    #[must_use]
    pub fn page(&self, node_id: &NodeId) -> Option<&SeoPage> {
        self.pages.get(node_id)
    }

    #[must_use]
    pub fn with_existing_links_from_graph<F>(mut self, graph: &Graph, mut is_link: F) -> Self
    where
        F: FnMut(&Edge) -> bool,
    {
        for edge in graph.edges().iter().filter(|edge| is_link(edge)) {
            if let Some(source) = self.pages.get_mut(&edge.source) {
                source.existing_targets.insert(edge.target.clone());
            }
        }
        self
    }

    fn profiles(&self, source: &NodeId, target: &NodeId) -> Option<(&SeoPage, &SeoPage)> {
        Some((self.pages.get(source)?, self.pages.get(target)?))
    }
}

impl LinkPolicy for SeoLinkPolicy {
    fn id(&self) -> &'static str {
        "seo-v1"
    }

    fn validate(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<()> {
        for vector in vectors {
            if !self.pages.contains_key(vector.node_id()) {
                return Err(SemanticError::MissingSeoProfile {
                    node: vector.node_id().to_string(),
                });
            }
        }
        for node in self.pages.keys() {
            if graph.node(node.as_str()).is_none() {
                return Err(SemanticError::SeoProfileMissingGraphNode {
                    node: node.to_string(),
                });
            }
        }
        Ok(())
    }

    fn allows(&self, source: &NodeId, target: &NodeId) -> bool {
        if source == target {
            return false;
        }
        let Some((source_page, target_page)) = self.profiles(source, target) else {
            return false;
        };
        if !source_page.eligibility.source
            || !target_page.eligibility.target
            || source_page.site != target_page.site
            || source_page.canonical == target_page.canonical
            || source_page.existing_targets.contains(target)
        {
            return false;
        }
        self.allow_cross_language || source_page.language == target_page.language
    }

    fn annotate_edge(&self, edge: Edge, source: &NodeId, target: &NodeId) -> Result<Edge> {
        let Some((source_page, target_page)) = self.profiles(source, target) else {
            return Err(SemanticError::MissingSeoProfile {
                node: target.to_string(),
            });
        };
        let mut edge = edge
            .with_attribute("policy", self.id())
            .with_attribute("recommendation", "internal_link")
            .with_attribute("site", source_page.site.clone())
            .with_attribute("source_canonical", source_page.canonical.clone())
            .with_attribute("target_canonical", target_page.canonical.clone())
            .with_attribute("target_cornerstone", target_page.signals.cornerstone)
            .with_attribute("target_orphan", target_page.signals.orphan)
            .with_attribute("target_priority", target_page.signals.target_priority)
            .with_attribute("existing_link", false);
        if let Some(language) = source_page.language() {
            edge = edge.with_attribute("source_language", language);
        }
        if let Some(language) = target_page.language() {
            edge = edge.with_attribute("target_language", language);
        }
        Ok(edge)
    }
}
