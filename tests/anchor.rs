use weavatrix_graph::{Graph, GraphBuilder, Node, NodeId, NodeKind};
use weavatrix_semantic::{
    AnchorCandidate, AnchorConfig, AnchorMatcher, LinkConfig, SelectionMode, SemanticError,
    SemanticLinker, SemanticVector, SeoLinkPolicy, SeoPage,
};

fn id(value: &str) -> NodeId {
    NodeId::new(value).unwrap()
}

fn vector(node: &str, values: &[f32]) -> SemanticVector {
    SemanticVector::new(node, values.to_vec()).unwrap()
}

fn fixture() -> (
    Graph,
    Vec<SemanticVector>,
    SeoLinkPolicy,
    weavatrix_semantic::SemanticLinkReport,
) {
    let mut builder = GraphBuilder::new();
    let kind = NodeKind::custom("page").unwrap();
    for node in ["page:/source", "page:/target"] {
        builder
            .add_node(Node::new(node, node, kind.clone()).unwrap())
            .unwrap();
    }
    let graph = builder.build().unwrap();
    let vectors = vec![
        vector("page:/source", &[0.8, 0.2, 0.0]),
        vector("page:/target", &[1.0, 0.0, 0.0]),
    ];
    let policy = SeoLinkPolicy::new([
        SeoPage::new(id("page:/source"), "example.com", "/source")
            .unwrap()
            .with_language("en")
            .unwrap(),
        SeoPage::new(id("page:/target"), "example.com", "/target")
            .unwrap()
            .with_language("en")
            .unwrap()
            .with_source_eligible(false),
    ])
    .unwrap();
    let report = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.7, 1).with_selection(SelectionMode::Directed),
    )
    .unwrap()
    .link_with_policy(&graph, &vectors, &policy)
    .unwrap();
    (graph, vectors, policy, report)
}

#[test]
fn ranks_existing_source_text_for_each_directed_link() {
    let (_graph, vectors, _policy, report) = fixture();
    let candidates = [
        AnchorCandidate::new(
            id("page:/source"),
            "main:p1",
            "Rust package manager",
            "The Rust package manager Cargo is also a build tool.",
            vec![0.99, 0.01, 0.0],
        )
        .unwrap(),
        AnchorCandidate::new(
            id("page:/source"),
            "main:p2",
            "weeknight recipes",
            "These weeknight recipes are quick to prepare.",
            vec![0.0, 1.0, 0.0],
        )
        .unwrap(),
    ];
    let matcher = AnchorMatcher::new(AnchorConfig::new("embedding-v1", 0.8, 2)).unwrap();

    let anchors = matcher.match_links(&report, &vectors, &candidates).unwrap();

    assert_eq!(anchors.candidate_count(), 2);
    assert_eq!(anchors.comparisons(), 2);
    assert_eq!(anchors.matched_link_count(), 1);
    let link = &anchors.links()[0];
    assert_eq!(link.source(), &id("page:/source"));
    assert_eq!(link.target(), &id("page:/target"));
    assert_eq!(link.suggestions().len(), 1);
    assert_eq!(link.suggestions()[0].locator(), "main:p1");
    assert_eq!(link.suggestions()[0].anchor_text(), "Rust package manager");
    assert!(link.suggestions()[0].similarity() > 0.99);
}

#[test]
fn stable_locator_order_breaks_equal_similarity_ties() {
    let (_graph, vectors, _policy, report) = fixture();
    let candidates = [
        AnchorCandidate::new(
            id("page:/source"),
            "main:z",
            "later",
            "The later passage has the same semantic context.",
            vec![1.0, 0.0, 0.0],
        )
        .unwrap(),
        AnchorCandidate::new(
            id("page:/source"),
            "main:a",
            "earlier",
            "The earlier passage has the same semantic context.",
            vec![1.0, 0.0, 0.0],
        )
        .unwrap(),
    ];
    let matcher = AnchorMatcher::new(AnchorConfig::new("embedding-v1", 0.8, 1)).unwrap();

    let anchors = matcher.match_links(&report, &vectors, &candidates).unwrap();

    assert_eq!(anchors.links()[0].suggestions()[0].locator(), "main:a");
}

#[test]
fn rejects_duplicate_locations_dimension_mismatch_and_model_mismatch() {
    let (_graph, vectors, _policy, report) = fixture();
    let first = AnchorCandidate::new(
        id("page:/source"),
        "main:p1",
        "anchor",
        "Context text with anchor.",
        vec![1.0, 0.0, 0.0],
    )
    .unwrap();
    let duplicate = first.clone();
    let matcher = AnchorMatcher::new(AnchorConfig::new("embedding-v1", 0.8, 1)).unwrap();
    assert!(matches!(
        matcher.match_links(&report, &vectors, &[first, duplicate]),
        Err(SemanticError::DuplicateAnchorCandidate { .. })
    ));

    let wrong_dimension = AnchorCandidate::new(
        id("page:/source"),
        "main:p2",
        "anchor",
        "Context text with anchor.",
        vec![1.0, 0.0],
    )
    .unwrap();
    assert!(matches!(
        matcher.match_links(&report, &vectors, &[wrong_dimension]),
        Err(SemanticError::AnchorDimensionMismatch { .. })
    ));

    let wrong_model = AnchorMatcher::new(AnchorConfig::new("embedding-v2", 0.8, 1)).unwrap();
    assert!(matches!(
        wrong_model.match_links(&report, &vectors, &[]),
        Err(SemanticError::AnchorModelMismatch { .. })
    ));

    assert!(matches!(
        AnchorCandidate::new(
            id("page:/source"),
            "main:p3",
            "missing phrase",
            "Different context.",
            vec![1.0, 0.0, 0.0],
        ),
        Err(SemanticError::AnchorTextOutsideContext { .. })
    ));
}
