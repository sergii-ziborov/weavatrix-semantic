mod seo_link;

use crate::{Result, SemanticError, SemanticVector};
pub use seo_link::SeoLinkPolicy;
use std::collections::BTreeSet;
use weavatrix_graph::{Edge, Graph, NodeId};

/// Eligibility and evidence policy applied after semantic candidate discovery.
///
/// Policies never change cosine similarity. They decide which directed
/// source-target recommendations are valid and annotate retained graph edges.
pub trait LinkPolicy {
    /// Stable policy identifier stored in reports and emitted edges.
    fn id(&self) -> &str;

    /// Validates policy coverage against the graph and semantic input.
    ///
    /// # Errors
    ///
    /// Returns an error when policy data is incomplete or inconsistent.
    fn validate(&self, graph: &Graph, vectors: &[SemanticVector]) -> Result<()>;

    /// Returns whether a directed source-target recommendation is eligible.
    fn allows(&self, source: &NodeId, target: &NodeId) -> bool;

    /// Adds policy-specific evidence to a retained edge.
    ///
    /// # Errors
    ///
    /// Implementations may reject inconsistent policy state.
    fn annotate_edge(&self, edge: Edge, _source: &NodeId, _target: &NodeId) -> Result<Edge> {
        Ok(edge.with_attribute("policy", self.id()))
    }
}

/// Unrestricted policy used by the general-purpose semantic linker methods.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllowAllPolicy;

impl LinkPolicy for AllowAllPolicy {
    fn id(&self) -> &'static str {
        "allow_all"
    }

    fn validate(&self, _graph: &Graph, _vectors: &[SemanticVector]) -> Result<()> {
        Ok(())
    }

    fn allows(&self, source: &NodeId, target: &NodeId) -> bool {
        source != target
    }
}

/// Explicit page metadata used to decide whether an SEO link is valid.
///
/// Crawling, canonical resolution, indexability, language detection, existing
/// link extraction, and authority analysis stay outside this crate. Their
/// results enter the semantic layer through this deterministic profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeoPage {
    node_id: NodeId,
    site: String,
    canonical: String,
    language: Option<String>,
    eligibility: SeoEligibility,
    existing_targets: BTreeSet<NodeId>,
    signals: SeoSignals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SeoEligibility {
    source: bool,
    target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SeoSignals {
    cornerstone: bool,
    orphan: bool,
    target_priority: u32,
}

impl SeoPage {
    /// Creates an indexable source and target profile.
    ///
    /// # Errors
    ///
    /// Returns an error for empty or whitespace-padded site/canonical values.
    pub fn new(
        node_id: NodeId,
        site: impl Into<String>,
        canonical: impl Into<String>,
    ) -> Result<Self> {
        let site = site.into();
        let canonical = canonical.into();
        validate_page_text(&node_id, "site", &site)?;
        validate_page_text(&node_id, "canonical", &canonical)?;
        Ok(Self {
            node_id,
            site,
            canonical,
            language: None,
            eligibility: SeoEligibility {
                source: true,
                target: true,
            },
            existing_targets: BTreeSet::new(),
            signals: SeoSignals {
                cornerstone: false,
                orphan: false,
                target_priority: 0,
            },
        })
    }

    /// Graph node represented by this profile.
    #[must_use]
    pub const fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Caller-normalized site identity used to prevent cross-site links.
    #[must_use]
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Caller-normalized canonical content identity.
    #[must_use]
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Optional caller-normalized content language.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Whether the page may be the source of a recommendation.
    #[must_use]
    pub const fn source_eligible(&self) -> bool {
        self.eligibility.source
    }

    /// Whether the page may be the target of a recommendation.
    #[must_use]
    pub const fn target_eligible(&self) -> bool {
        self.eligibility.target
    }

    /// Whether the target is designated as cornerstone content.
    #[must_use]
    pub const fn cornerstone(&self) -> bool {
        self.signals.cornerstone
    }

    /// Whether the target currently has no known internal inbound links.
    #[must_use]
    pub const fn orphan(&self) -> bool {
        self.signals.orphan
    }

    /// Caller-computed target priority exposed without changing similarity.
    #[must_use]
    pub const fn target_priority(&self) -> u32 {
        self.signals.target_priority
    }

    /// Adds a normalized content language.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty or whitespace-padded language.
    pub fn with_language(mut self, language: impl Into<String>) -> Result<Self> {
        let language = language.into();
        if language.is_empty() {
            return Err(SemanticError::EmptySeoLanguage {
                node: self.node_id.to_string(),
            });
        }
        if language.trim() != language {
            return Err(SemanticError::SeoLanguageHasSurroundingWhitespace {
                node: self.node_id.to_string(),
            });
        }
        self.language = Some(language);
        Ok(self)
    }

    /// Enables or disables using the page as a recommendation source.
    #[must_use]
    pub const fn with_source_eligible(mut self, eligible: bool) -> Self {
        self.eligibility.source = eligible;
        self
    }

    /// Enables or disables using the page as a recommendation target.
    #[must_use]
    pub const fn with_target_eligible(mut self, eligible: bool) -> Self {
        self.eligibility.target = eligible;
        self
    }

    /// Records an already-existing directed internal link to suppress.
    #[must_use]
    pub fn with_existing_target(mut self, target: NodeId) -> Self {
        self.existing_targets.insert(target);
        self
    }

    /// Marks the page as cornerstone content for downstream prioritization.
    #[must_use]
    pub const fn with_cornerstone(mut self, cornerstone: bool) -> Self {
        self.signals.cornerstone = cornerstone;
        self
    }

    /// Marks the page as orphaned for downstream prioritization.
    #[must_use]
    pub const fn with_orphan(mut self, orphan: bool) -> Self {
        self.signals.orphan = orphan;
        self
    }

    /// Adds a caller-computed target priority without rewriting similarity.
    #[must_use]
    pub const fn with_target_priority(mut self, priority: u32) -> Self {
        self.signals.target_priority = priority;
        self
    }
}

fn validate_page_text(node: &NodeId, field: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(match field {
            "site" => SemanticError::EmptySeoSite {
                node: node.to_string(),
            },
            _ => SemanticError::EmptySeoCanonical {
                node: node.to_string(),
            },
        });
    }
    if value.trim() != value {
        return Err(match field {
            "site" => SemanticError::SeoSiteHasSurroundingWhitespace {
                node: node.to_string(),
            },
            _ => SemanticError::SeoCanonicalHasSurroundingWhitespace {
                node: node.to_string(),
            },
        });
    }
    Ok(())
}
