use weavatrix_graph::{
    AttributeValue, Confidence, Edge, EdgeKind, EvidenceKind, Graph, GraphBuilder, Node, NodeId,
    NodeKind, Provenance,
};
use weavatrix_semantic::{
    CandidateBackend, LinkConfig, SEMANTIC_EDGE_KIND, SEMANTIC_EXTRACTOR, SelectionMode,
    SemanticError, SemanticLinker, SemanticVector,
};

fn page_graph(ids: &[&str]) -> Graph {
    let mut builder = GraphBuilder::new();
    for &id in ids {
        builder
            .add_node(Node::new(id, id, NodeKind::custom("page").unwrap()).unwrap())
            .unwrap();
    }
    builder.build().unwrap()
}

fn vector(id: &str, values: &[f32]) -> SemanticVector {
    SemanticVector::new(id, values.to_vec()).unwrap()
}

#[test]
fn links_mutual_neighbors_with_evidence_and_metadata() {
    let graph = page_graph(&["page:/rust", "page:/cargo", "page:/recipes"]);
    let vectors = vec![
        vector("page:/rust", &[1.0, 0.0]),
        vector("page:/cargo", &[0.98, 0.10]),
        vector("page:/recipes", &[0.0, 1.0]),
    ];
    let linker = SemanticLinker::new(LinkConfig::new("embedding-v1", 0.90, 1)).unwrap();

    let report = linker.link(&graph, &vectors).unwrap();

    assert_eq!(report.vector_count(), 3);
    assert_eq!(report.dimension(), 2);
    assert_eq!(report.comparisons(), 3);
    assert_eq!(report.pair_count(), 1);
    assert_eq!(report.candidate_backend(), CandidateBackend::Exact);
    assert_eq!(report.edges().len(), 2);
    for edge in report.edges() {
        assert_eq!(edge.kind.as_str(), SEMANTIC_EDGE_KIND);
        assert_eq!(edge.provenance.extractor, SEMANTIC_EXTRACTOR);
        assert_eq!(edge.provenance.evidence, EvidenceKind::Inferred);
        assert_eq!(edge.provenance.confidence, Confidence::Low);
        assert_eq!(
            edge.attributes.get("model"),
            Some(&AttributeValue::String("embedding-v1".to_owned()))
        );
        assert_eq!(
            edge.attributes.get("candidate_backend"),
            Some(&AttributeValue::String("exact".to_owned()))
        );
        assert_eq!(
            edge.attributes.get("candidate_exact"),
            Some(&AttributeValue::Bool(true))
        );
        assert!(matches!(
            edge.attributes.get("similarity"),
            Some(AttributeValue::Float(score)) if score.get() > 0.99
        ));
    }
}

#[test]
fn output_is_independent_of_vector_input_order() {
    let graph = page_graph(&["page:/a", "page:/b", "page:/c"]);
    let vectors = vec![
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.98, 0.10]),
        vector("page:/c", &[0.0, 1.0]),
    ];
    let reversed = vectors.iter().cloned().rev().collect::<Vec<_>>();
    let linker = SemanticLinker::new(LinkConfig::new("embedding-v1", 0.80, 2)).unwrap();

    assert_eq!(
        linker.link(&graph, &vectors).unwrap().edges(),
        linker.link(&graph, &reversed).unwrap().edges()
    );
}

#[test]
fn union_mode_retains_one_sided_top_k_choices() {
    let graph = page_graph(&["page:/a", "page:/b", "page:/c"]);
    let vectors = vec![
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.9848, 0.1736]),
        vector("page:/c", &[0.9397, 0.3420]),
    ];
    let mutual = SemanticLinker::new(LinkConfig::new("embedding-v1", 0.90, 1)).unwrap();
    let union = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 0.90, 1).with_selection(SelectionMode::Union),
    )
    .unwrap();

    assert_eq!(mutual.link(&graph, &vectors).unwrap().pair_count(), 1);
    assert_eq!(union.link(&graph, &vectors).unwrap().pair_count(), 2);
}

#[test]
fn rejects_missing_duplicate_and_mismatched_vectors() {
    let graph = page_graph(&["page:/a", "page:/b"]);
    let linker = SemanticLinker::new(LinkConfig::new("embedding-v1", 0.80, 2)).unwrap();

    assert!(matches!(
        linker.link(&graph, &[vector("page:/missing", &[1.0, 0.0])]),
        Err(SemanticError::MissingGraphNode { .. })
    ));
    assert!(matches!(
        linker.link(
            &graph,
            &[
                vector("page:/a", &[1.0, 0.0]),
                vector("page:/a", &[0.9, 0.1])
            ]
        ),
        Err(SemanticError::DuplicateNode { .. })
    ));
    assert!(matches!(
        linker.link(
            &graph,
            &[
                vector("page:/a", &[1.0, 0.0]),
                vector("page:/b", &[1.0, 0.0, 0.0])
            ]
        ),
        Err(SemanticError::DimensionMismatch { .. })
    ));
}

#[test]
fn rejects_invalid_vectors_and_configuration() {
    assert!(matches!(
        SemanticVector::new("page:/a", Vec::new()),
        Err(SemanticError::EmptyVector { .. })
    ));
    assert!(matches!(
        SemanticVector::new("page:/a", vec![0.0, 0.0]),
        Err(SemanticError::ZeroVector { .. })
    ));
    assert!(matches!(
        SemanticVector::new("page:/a", vec![f32::NAN]),
        Err(SemanticError::NonFiniteVectorValue { .. })
    ));
    assert!(matches!(
        SemanticLinker::new(LinkConfig::new("", 0.8, 2)),
        Err(SemanticError::EmptyModel)
    ));
    assert!(matches!(
        SemanticLinker::new(LinkConfig::new("model", 1.1, 2)),
        Err(SemanticError::InvalidSimilarityThreshold)
    ));
    assert!(matches!(
        SemanticLinker::new(LinkConfig::new("model", 0.8, 0)),
        Err(SemanticError::ZeroTopK)
    ));
    assert!(matches!(
        SemanticLinker::new(LinkConfig::new("model", 0.8, 2).with_max_vectors(0)),
        Err(SemanticError::ZeroMaxVectors)
    ));
}

#[test]
fn has_no_default_vector_count_limit_but_honors_caller_bound() {
    assert_eq!(
        LinkConfig::new("embedding-v1", 0.8, 2).max_vectors(),
        usize::MAX
    );

    let graph = page_graph(&["page:/a", "page:/b", "page:/c"]);
    let vectors = vec![
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.9, 0.1]),
        vector("page:/c", &[0.8, 0.2]),
    ];
    let linker =
        SemanticLinker::new(LinkConfig::new("embedding-v1", 0.8, 2).with_max_vectors(2)).unwrap();

    assert!(matches!(
        linker.link(&graph, &vectors),
        Err(SemanticError::TooManyVectors {
            count: 3,
            maximum: 2
        })
    ));
}

#[test]
fn equal_scores_use_node_id_as_the_stable_top_k_tiebreaker() {
    let graph = page_graph(&["page:/d", "page:/c", "page:/b", "page:/a"]);
    let vectors = vec![
        vector("page:/d", &[1.0, 0.0]),
        vector("page:/c", &[1.0, 0.0]),
        vector("page:/b", &[1.0, 0.0]),
        vector("page:/a", &[1.0, 0.0]),
    ];
    let mutual = SemanticLinker::new(LinkConfig::new("embedding-v1", 1.0, 1)).unwrap();
    let union = SemanticLinker::new(
        LinkConfig::new("embedding-v1", 1.0, 1).with_selection(SelectionMode::Union),
    )
    .unwrap();

    assert_eq!(mutual.link(&graph, &vectors).unwrap().pair_count(), 1);
    assert_eq!(union.link(&graph, &vectors).unwrap().pair_count(), 3);
}

#[test]
fn relink_replaces_only_its_own_previous_edges() {
    let mut builder = GraphBuilder::new();
    for id in ["page:/a", "page:/b"] {
        builder
            .add_node(Node::new(id, id, NodeKind::custom("page").unwrap()).unwrap())
            .unwrap();
    }
    builder
        .add_edge(Edge::new(
            NodeId::new("page:/a").unwrap(),
            NodeId::new("page:/b").unwrap(),
            EdgeKind::custom("links_to").unwrap(),
            Provenance::new(
                "html-link-extractor",
                EvidenceKind::Extracted,
                Confidence::Exact,
            )
            .unwrap(),
        ))
        .unwrap();
    let graph = builder.build().unwrap();
    let vectors = vec![
        vector("page:/a", &[1.0, 0.0]),
        vector("page:/b", &[0.99, 0.01]),
    ];

    let first = SemanticLinker::new(LinkConfig::new("embedding-v1", 0.90, 1)).unwrap();
    let once = first.relink(&graph, &vectors).unwrap();
    let second = SemanticLinker::new(LinkConfig::new("embedding-v2", 0.90, 1)).unwrap();
    let twice = second.relink(&once, &vectors).unwrap();

    assert_eq!(once.edge_count(), 3);
    assert_eq!(twice.edge_count(), 3);
    assert_eq!(
        twice
            .edges()
            .iter()
            .filter(|edge| edge.provenance.extractor == "html-link-extractor")
            .count(),
        1
    );
    for edge in twice
        .edges()
        .iter()
        .filter(|edge| edge.provenance.extractor == SEMANTIC_EXTRACTOR)
    {
        assert_eq!(
            edge.attributes.get("model"),
            Some(&AttributeValue::String("embedding-v2".to_owned()))
        );
    }
}
