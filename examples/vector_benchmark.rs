use std::collections::BTreeSet;
use std::error::Error;
use std::time::{Duration, Instant};
use weavatrix_graph::{Graph, GraphBuilder, Node, NodeId, NodeKind};
use weavatrix_semantic::{
    LinkConfig, SelectionMode, SemanticLinkReport, SemanticLinker, SemanticVector,
    VectorCandidateConfig, VectorSemanticLinker,
};

const DIMENSIONS: usize = 384;
const TOP_K: usize = 8;
const RUNS: usize = 3;

type DynResult<T> = Result<T, Box<dyn Error>>;
type PairSet = BTreeSet<(NodeId, NodeId)>;

fn main() -> DynResult<()> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "10000".to_owned())
        .parse::<usize>()?;
    if count < 2 {
        return Err("benchmark vector count must be at least two".into());
    }

    let values = clustered_vectors(count, DIMENSIONS);
    let (graph, vectors) = graph_and_vectors(&values)?;
    let config =
        LinkConfig::new("synthetic-benchmark", 0.0, TOP_K).with_selection(SelectionMode::Union);
    let expected = pairs(&SemanticLinker::new(config.clone())?.link(&graph, &vectors)?);
    let linker = VectorSemanticLinker::new(config, VectorCandidateConfig::new(DIMENSIONS))?;

    let _ = linker.link(&graph, &vectors)?;
    let mut times = Vec::with_capacity(RUNS);
    let mut recalls = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let report = linker.link(&graph, &vectors)?;
        times.push(started.elapsed());
        recalls.push(pair_recall(&expected, &pairs(&report))?);
    }

    println!("vectors={count} dimensions={DIMENSIONS} top_k={TOP_K} selection=union runs={RUNS}");
    println!(
        "median_vector_semantic_ms={:.3} median_exact_pair_recall={:.6}",
        milliseconds(median_duration(&mut times)),
        median_f64(&mut recalls),
    );
    Ok(())
}

fn graph_and_vectors(values: &[Vec<f32>]) -> DynResult<(Graph, Vec<SemanticVector>)> {
    let mut builder = GraphBuilder::with_capacity(values.len(), 0);
    let kind = NodeKind::custom("page")?;
    let mut vectors = Vec::with_capacity(values.len());
    for (index, vector) in values.iter().enumerate() {
        let id = format!("page:{index:08}");
        builder.add_node(Node::new(id.clone(), id.clone(), kind.clone())?)?;
        vectors.push(SemanticVector::new(id, vector.clone())?);
    }
    Ok((builder.build()?, vectors))
}

fn pairs(report: &SemanticLinkReport) -> PairSet {
    report
        .edges()
        .iter()
        .map(|edge| {
            if edge.source < edge.target {
                (edge.source.clone(), edge.target.clone())
            } else {
                (edge.target.clone(), edge.source.clone())
            }
        })
        .collect()
}

fn pair_recall(expected: &PairSet, actual: &PairSet) -> DynResult<f64> {
    let hits = u32::try_from(expected.intersection(actual).count())?;
    let total = u32::try_from(expected.len())?;
    Ok(f64::from(hits) / f64::from(total))
}

fn clustered_vectors(count: usize, dimensions: usize) -> Vec<Vec<f32>> {
    let cluster_count = count.min(64);
    let mut random = XorShift64(0x7f4a_7c15_9e37_79b9);
    let centroids = (0..cluster_count)
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

fn median_duration(values: &mut [Duration]) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    values[values.len() / 2]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_signed(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let high = u16::try_from(self.0 >> 48).expect("shifted value fits u16");
        (f32::from(high) / f32::from(u16::MAX)).mul_add(2.0, -1.0)
    }
}
