use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::time::{Duration, Instant};
use weavatrix_graph::{Graph, GraphBuilder, Node, NodeId, NodeKind};
use weavatrix_semantic::{
    LinkConfig, LinkPolicy, SelectionMode, SemanticLinkReport, SemanticLinker, SemanticVector,
    SeoLinkPolicy, SeoPage, VectorCandidateConfig, VectorSemanticLinker,
};

const DIMENSIONS: usize = 384;
const TOP_K: usize = 8;
const RUNS: usize = 3;
const ORACLE_LIMIT: usize = 1_500;
const SITE_COUNT: usize = 4;
const LANGUAGE_COUNT: usize = 2;

type DynResult<T> = Result<T, Box<dyn Error>>;
type DirectedSet = BTreeSet<(NodeId, NodeId)>;

fn main() -> DynResult<()> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "10000".to_owned())
        .parse::<usize>()?;
    if count < 2 {
        return Err("benchmark page count must be at least two".into());
    }

    let (graph, vectors, policy) = corpus(count)?;
    let config =
        LinkConfig::new("seo-benchmark", 0.0, TOP_K).with_selection(SelectionMode::Directed);
    let linker = VectorSemanticLinker::new(
        config.clone(),
        VectorCandidateConfig::new(DIMENSIONS).with_candidate_pool_multiplier(2),
    )?;

    let _warmup = linker.link_with_policy(&graph, &vectors, &policy)?;
    let mut times = Vec::with_capacity(RUNS);
    let mut reports = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let report = linker.link_with_policy(&graph, &vectors, &policy)?;
        times.push(started.elapsed());
        reports.push(report);
    }
    let report = reports.pop().ok_or("benchmark produced no reports")?;
    let forbidden = report
        .edges()
        .iter()
        .filter(|edge| !policy.allows(&edge.source, &edge.target))
        .count();
    let over_budget = sources_over_budget(&report, TOP_K);

    let oracle_count = count.min(ORACLE_LIMIT);
    let (oracle_graph, oracle_vectors, oracle_policy) = corpus(oracle_count)?;
    let expected = directed_edges(&SemanticLinker::new(config.clone())?.link_with_policy(
        &oracle_graph,
        &oracle_vectors,
        &oracle_policy,
    )?);
    let actual = directed_edges(
        &VectorSemanticLinker::new(
            config,
            VectorCandidateConfig::new(DIMENSIONS).with_candidate_pool_multiplier(2),
        )?
        .link_with_policy(&oracle_graph, &oracle_vectors, &oracle_policy)?,
    );
    let recall = recall(&expected, &actual)?;

    println!("pages={count} dimensions={DIMENSIONS} top_k={TOP_K} selection=directed runs={RUNS}");
    println!(
        "median_vector_semantic_ms={:.3} emitted_edges={} forbidden_edges={} sources_over_budget={}",
        milliseconds(median_duration(&mut times)),
        report.edge_count(),
        forbidden,
        over_budget
    );
    println!(
        "oracle_pages={oracle_count} exact_directed_edges={} approximate_directed_edges={} exact_edge_recall={recall:.6}",
        expected.len(),
        actual.len()
    );

    if forbidden != 0 || over_budget != 0 || recall < 0.995 {
        return Err("SEO semantic benchmark acceptance criteria failed".into());
    }
    Ok(())
}

fn corpus(count: usize) -> DynResult<(Graph, Vec<SemanticVector>, SeoLinkPolicy)> {
    let values = clustered_vectors(count, DIMENSIONS);
    let kind = NodeKind::custom("page")?;
    let mut builder = GraphBuilder::with_capacity(count, 0);
    let mut vectors = Vec::with_capacity(count);
    let mut pages = Vec::with_capacity(count);

    for (index, values) in values.into_iter().enumerate() {
        let id = page_id(index)?;
        builder.add_node(Node::new(id.to_string(), id.to_string(), kind.clone())?)?;
        vectors.push(SemanticVector::new(id.to_string(), values)?);

        let site = format!("site-{}", index % SITE_COUNT);
        let language = ["en", "de"][(index / SITE_COUNT) % LANGUAGE_COUNT];
        let canonical_index = if index >= SITE_COUNT * LANGUAGE_COUNT && index % 113 == 0 {
            index - SITE_COUNT * LANGUAGE_COUNT
        } else {
            index
        };
        let mut page = SeoPage::new(id, site, format!("/article/{canonical_index}"))?
            .with_language(language)?
            .with_source_eligible(index % 89 != 0)
            .with_target_eligible(index % 97 != 0)
            .with_cornerstone(index % 101 == 0)
            .with_orphan(index % 67 == 0)
            .with_target_priority(u32::try_from(index % 100)?);
        let existing_index = index + SITE_COUNT * LANGUAGE_COUNT;
        if existing_index < count {
            page = page.with_existing_target(page_id(existing_index)?);
        }
        pages.push(page);
    }

    Ok((builder.build()?, vectors, SeoLinkPolicy::new(pages)?))
}

fn page_id(index: usize) -> DynResult<NodeId> {
    Ok(NodeId::new(format!("page:{index:08}"))?)
}

fn directed_edges(report: &SemanticLinkReport) -> DirectedSet {
    report
        .edges()
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .collect()
}

fn sources_over_budget(report: &SemanticLinkReport, top_k: usize) -> usize {
    let mut counts = BTreeMap::<&NodeId, usize>::new();
    for edge in report.edges() {
        *counts.entry(&edge.source).or_default() += 1;
    }
    counts.values().filter(|&&count| count > top_k).count()
}

fn recall(expected: &DirectedSet, actual: &DirectedSet) -> DynResult<f64> {
    if expected.is_empty() {
        return Ok(1.0);
    }
    let hits = u32::try_from(expected.intersection(actual).count())?;
    let total = u32::try_from(expected.len())?;
    Ok(f64::from(hits) / f64::from(total))
}

fn clustered_vectors(count: usize, dimensions: usize) -> Vec<Vec<f32>> {
    let cluster_count = count.min(64);
    let mut random = XorShift64(0x8ab4_6d21_5f09_c37e);
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
            let mut vector = centroids[index % cluster_count]
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
