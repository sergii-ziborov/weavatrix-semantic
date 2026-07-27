use weavatrix_graph::{
    AttributeValue, Confidence, Edge, EdgeKind, EvidenceKind, Graph, GraphBuilder, Node, NodeId,
    NodeKind, Provenance,
};
use weavatrix_semantic::{
    LinkConfig, LinkPolicy, SelectionMode, SemanticError, SemanticLinker, SemanticVector,
    SeoLinkPolicy, SeoPage,
};

fn node_id(value: &str) -> NodeId {
    NodeId::new(value).unwrap()
}

fn graph(ids: &[&str]) -> Graph {
    let kind = NodeKind::custom("page").unwrap();
    let mut builder = GraphBuilder::new();
    for &id in ids {
        builder
            .add_node(Node::new(id, id, kind.clone()).unwrap())
            .unwrap();
    }
    builder.build().unwrap()
}

fn vector(id: &str, values: &[f32]) -> SemanticVector {
    SemanticVector::new(id, values.to_vec()).unwrap()
}

fn page(id: &str, site: &str, canonical: &str, language: &str) -> SeoPage {
    SeoPage::new(node_id(id), site, canonical)
        .unwrap()
        .with_language(language)
        .unwrap()
}

#[test]
fn directed_mode_keeps_only_eligible_source_choices() {
    let graph = graph(&["page:/source", "page:/target"]);
    let vectors = [
        vector("page:/source", &[1.0, 0.0]),
        vector("page:/target", &[0.99, 0.01]),
    ];
    let policy = SeoLinkPolicy::new([
        page("page:/source", "example.com", "/source", "en"),
        page("page:/target", "example.com", "/target", "en")
            .with_source_eligible(false)
            .with_cornerstone(true)
            .with_orphan(true)
            .with_target_priority(90),
    ])
    .unwrap();
    let linker = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, 1).with_selection(SelectionMode::Directed),
    )
    .unwrap();

    let report = linker.link_with_policy(&graph, &vectors, &policy).unwrap();

    assert_eq!(report.policy_id(), "seo-v1");
    assert_eq!(report.pair_count(), 1);
    assert_eq!(report.edge_count(), 1);
    let edge = &report.edges()[0];
    assert_eq!(edge.source, node_id("page:/source"));
    assert_eq!(edge.target, node_id("page:/target"));
    assert_eq!(
        edge.attributes.get("recommendation"),
        Some(&AttributeValue::String("internal_link".to_owned()))
    );
    assert_eq!(
        edge.attributes.get("target_cornerstone"),
        Some(&AttributeValue::Bool(true))
    );
    assert_eq!(
        edge.attributes.get("target_orphan"),
        Some(&AttributeValue::Bool(true))
    );
    assert_eq!(
        edge.attributes.get("target_priority"),
        Some(&AttributeValue::Unsigned(90))
    );
}

#[test]
fn seo_policy_blocks_invalid_and_existing_internal_links() {
    let ids = [
        "page:/source",
        "page:/existing",
        "page:/noindex",
        "page:/fr",
        "page:/other-site",
        "page:/duplicate",
        "page:/eligible",
    ];
    let graph = graph(&ids);
    let vectors = ids
        .iter()
        .map(|id| vector(id, &[1.0, 0.01]))
        .collect::<Vec<_>>();
    let source = page("page:/source", "example.com", "/source", "en")
        .with_existing_target(node_id("page:/existing"));
    let policy = SeoLinkPolicy::new([
        source,
        page("page:/existing", "example.com", "/existing", "en"),
        page("page:/noindex", "example.com", "/noindex", "en").with_target_eligible(false),
        page("page:/fr", "example.com", "/fr", "fr"),
        page("page:/other-site", "other.example", "/other-site", "en"),
        page("page:/duplicate", "example.com", "/source", "en"),
        page("page:/eligible", "example.com", "/eligible", "en"),
    ])
    .unwrap();
    let linker = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, ids.len()).with_selection(SelectionMode::Directed),
    )
    .unwrap();

    let report = linker.link_with_policy(&graph, &vectors, &policy).unwrap();
    let source_id = node_id("page:/source");
    let targets = report
        .edges()
        .iter()
        .filter(|edge| edge.source == source_id)
        .map(|edge| edge.target.as_str())
        .collect::<Vec<_>>();

    assert_eq!(targets, vec!["page:/eligible"]);
    assert!(
        report
            .edges()
            .iter()
            .all(|edge| policy.allows(&edge.source, &edge.target))
    );
}

#[test]
fn cross_language_recommendations_require_explicit_opt_in() {
    let graph = graph(&["page:/en", "page:/fr"]);
    let vectors = [
        vector("page:/en", &[1.0, 0.0]),
        vector("page:/fr", &[0.99, 0.01]),
    ];
    let pages = [
        page("page:/en", "example.com", "/en", "en"),
        page("page:/fr", "example.com", "/fr", "fr"),
    ];
    let linker = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, 1).with_selection(SelectionMode::Directed),
    )
    .unwrap();

    let strict = SeoLinkPolicy::new(pages.clone()).unwrap();
    assert!(
        linker
            .link_with_policy(&graph, &vectors, &strict)
            .unwrap()
            .edges()
            .is_empty()
    );

    let multilingual = SeoLinkPolicy::new(pages).unwrap().with_cross_language(true);
    assert_eq!(
        linker
            .link_with_policy(&graph, &vectors, &multilingual)
            .unwrap()
            .edge_count(),
        2
    );
}

#[test]
fn existing_link_evidence_can_be_imported_from_the_graph() {
    let mut builder = GraphBuilder::new();
    let kind = NodeKind::custom("page").unwrap();
    for id in ["page:/a", "page:/b"] {
        builder
            .add_node(Node::new(id, id, kind.clone()).unwrap())
            .unwrap();
    }
    builder
        .add_edge(Edge::new(
            node_id("page:/a"),
            node_id("page:/b"),
            EdgeKind::custom("links_to").unwrap(),
            Provenance::new("crawler", EvidenceKind::Extracted, Confidence::Exact).unwrap(),
        ))
        .unwrap();
    let graph = builder.build().unwrap();
    let vectors = [
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.99, 0.01]),
    ];
    let policy = SeoLinkPolicy::new([
        page("page:/a", "example.com", "/a", "en"),
        page("page:/b", "example.com", "/b", "en"),
    ])
    .unwrap()
    .with_existing_links_from_graph(&graph, |edge| edge.kind.as_str() == "links_to");
    let linker = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, 1).with_selection(SelectionMode::Directed),
    )
    .unwrap();

    let report = linker.link_with_policy(&graph, &vectors, &policy).unwrap();

    assert!(
        report
            .edges()
            .iter()
            .all(|edge| !(edge.source.as_str() == "page:/a" && edge.target.as_str() == "page:/b"))
    );
}

#[test]
fn seo_policy_fails_closed_on_incomplete_or_invalid_profiles() {
    let graph = graph(&["page:/a", "page:/b"]);
    let vectors = [
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.99, 0.01]),
    ];
    let incomplete = SeoLinkPolicy::new([page("page:/a", "example.com", "/a", "en")]).unwrap();
    let linker = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, 1).with_selection(SelectionMode::Directed),
    )
    .unwrap();
    assert!(matches!(
        linker.link_with_policy(&graph, &vectors, &incomplete),
        Err(SemanticError::MissingSeoProfile { node }) if node == "page:/b"
    ));

    assert!(matches!(
        SeoLinkPolicy::new([
            page("page:/a", "example.com", "/a", "en"),
            page("page:/a", "example.com", "/a-copy", "en"),
        ]),
        Err(SemanticError::DuplicateSeoProfile { .. })
    ));
    assert!(matches!(
        SeoPage::new(node_id("page:/a"), "", "/a"),
        Err(SemanticError::EmptySeoSite { .. })
    ));
    assert!(matches!(
        page("page:/a", "example.com", "/a", "en").with_language(" en "),
        Err(SemanticError::SeoLanguageHasSurroundingWhitespace { .. })
    ));
}

#[cfg(feature = "vector-search")]
#[test]
fn first_party_vector_candidates_respect_the_same_seo_policy() {
    use weavatrix_semantic::{VectorCandidateConfig, VectorSemanticLinker};

    let graph = graph(&["page:/a", "page:/b", "page:/blocked"]);
    let vectors = [
        vector("page:/a", &[1.0, 0.0, 0.0]),
        vector("page:/b", &[0.99, 0.01, 0.0]),
        vector("page:/blocked", &[0.999, 0.001, 0.0]),
    ];
    let policy = SeoLinkPolicy::new([
        page("page:/a", "example.com", "/a", "en"),
        page("page:/b", "example.com", "/b", "en"),
        page("page:/blocked", "example.com", "/blocked", "en").with_target_eligible(false),
    ])
    .unwrap();
    let linker = VectorSemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.9, 1).with_selection(SelectionMode::Directed),
        VectorCandidateConfig::new(3).with_candidate_pool_multiplier(3),
    )
    .unwrap();

    let report = linker.link_with_policy(&graph, &vectors, &policy).unwrap();

    assert!(
        report
            .edges()
            .iter()
            .all(|edge| policy.allows(&edge.source, &edge.target))
    );
    assert!(
        report
            .edges()
            .iter()
            .all(|edge| edge.target.as_str() != "page:/blocked")
    );
}
