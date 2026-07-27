#![cfg(feature = "vector-search")]

use std::collections::BTreeSet;
use weavatrix_graph::{AttributeValue, Graph, GraphBuilder, Node, NodeKind};
use weavatrix_semantic::{
    CandidateBackend, LinkConfig, SelectionMode, SemanticError, SemanticLinkReport, SemanticLinker,
    SemanticVector, VectorCandidateConfig, VectorIndexConfig, VectorSemanticLinker,
};

fn graph_and_vectors(values: &[&[f32]]) -> (Graph, Vec<SemanticVector>) {
    let mut builder = GraphBuilder::with_capacity(values.len(), 0);
    let kind = NodeKind::custom("page").unwrap();
    let mut vectors = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = format!("page:/{index}");
        builder
            .add_node(Node::new(id.clone(), id.clone(), kind.clone()).unwrap())
            .unwrap();
        vectors.push(SemanticVector::new(id, value.to_vec()).unwrap());
    }
    (builder.build().unwrap(), vectors)
}

fn pairs(report: &SemanticLinkReport) -> BTreeSet<(String, String)> {
    report
        .edges()
        .iter()
        .map(|edge| {
            let source = edge.source.as_str();
            let target = edge.target.as_str();
            if source < target {
                (source.to_owned(), target.to_owned())
            } else {
                (target.to_owned(), source.to_owned())
            }
        })
        .collect()
}

#[test]
fn first_party_candidates_preserve_semantic_contract() {
    let (graph, vectors) = graph_and_vectors(&[
        &[1.0, 0.0, 0.0],
        &[0.99, 0.05, 0.0],
        &[0.95, 0.15, 0.0],
        &[0.0, 1.0, 0.0],
        &[0.0, 0.99, 0.05],
        &[0.0, 0.95, 0.15],
    ]);
    let config = LinkConfig::new("embedding-v1", 0.8, 2).with_selection(SelectionMode::Union);
    let exact = SemanticLinker::new(config.clone()).unwrap();
    let vector = VectorSemanticLinker::new(config, VectorCandidateConfig::new(3)).unwrap();

    let exact_report = exact.link(&graph, &vectors).unwrap();
    let vector_report = vector.link(&graph, &vectors).unwrap();

    assert_eq!(pairs(&vector_report), pairs(&exact_report));
    assert_eq!(
        vector_report.candidate_backend(),
        CandidateBackend::WeavatrixVector
    );
    assert_eq!(vector_report.comparisons(), 0);
    for edge in vector_report.edges() {
        assert_eq!(
            edge.attributes.get("candidate_backend"),
            Some(&AttributeValue::String(
                "weavatrix_search_vector".to_owned()
            ))
        );
        assert_eq!(
            edge.attributes.get("candidate_exact"),
            Some(&AttributeValue::Bool(false))
        );
        assert!(matches!(
            edge.attributes.get("similarity"),
            Some(AttributeValue::Float(score)) if score.get() >= 0.8
        ));
    }
}

#[test]
fn rejects_invalid_vector_candidate_policy() {
    assert!(matches!(
        VectorSemanticLinker::new(
            LinkConfig::new("embedding-v1", 0.8, 2),
            VectorCandidateConfig::new(3).with_candidate_pool_multiplier(0),
        ),
        Err(SemanticError::ZeroCandidatePoolMultiplier)
    ));

    assert!(matches!(
        VectorSemanticLinker::new(
            LinkConfig::new("embedding-v1", 0.8, 2),
            VectorCandidateConfig::from_index_config(VectorIndexConfig::new(0)),
        ),
        Err(SemanticError::VectorSearch(_))
    ));
}

#[test]
fn semantic_pair_recall_stays_close_to_exact() {
    let values = clustered_vectors(512, 32);
    let borrowed = values.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let (graph, vectors) = graph_and_vectors(&borrowed);
    let config = LinkConfig::new("embedding-v1", 0.0, 8).with_selection(SelectionMode::Union);
    let exact = SemanticLinker::new(config.clone())
        .unwrap()
        .link(&graph, &vectors)
        .unwrap();
    let approximate = VectorSemanticLinker::new(config, VectorCandidateConfig::new(32))
        .unwrap()
        .link(&graph, &vectors)
        .unwrap();

    let expected = pairs(&exact);
    let actual = pairs(&approximate);
    let hits = u32::try_from(expected.intersection(&actual).count()).unwrap();
    let total = u32::try_from(expected.len()).unwrap();
    let recall = f64::from(hits) / f64::from(total);
    assert!(recall >= 0.999, "semantic-pair recall was {recall:.6}");
}

fn clustered_vectors(count: usize, dimensions: usize) -> Vec<Vec<f32>> {
    let mut random = XorShift64(0x7f4a_7c15_9e37_79b9);
    let centroids = (0..32)
        .map(|_| {
            let mut vector = (0..dimensions)
                .map(|_| random.next_signed())
                .collect::<Vec<_>>();
            normalize(&mut vector);
            vector
        })
        .collect::<Vec<_>>();

    (0..count)
        .map(|index| {
            let mut vector = centroids[index % centroids.len()]
                .iter()
                .map(|&value| value + 0.075 * random.next_signed())
                .collect::<Vec<_>>();
            normalize(&mut vector);
            vector
        })
        .collect()
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    for value in vector {
        *value /= norm;
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_signed(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let high = u16::try_from(self.0 >> 48).unwrap();
        (f32::from(high) / f32::from(u16::MAX)).mul_add(2.0, -1.0)
    }
}
