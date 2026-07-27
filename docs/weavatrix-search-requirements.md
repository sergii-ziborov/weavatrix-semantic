# Weavatrix Search requirements for semantic linking

Status: the immutable candidate contract is published in
`weavatrix-search-vector` 0.2.0. Later vector APIs may add traversal filtering
and other storage/query capabilities, but Semantic depends only on the stable
published contract.

## Purpose

`weavatrix-search-vector` provides a first-party, reusable vector-candidate engine.
It finds likely nearest vector keys quickly. It does not decide which
relationships are semantically valid and it does not emit graph edges.

The immediate consumer is `weavatrix-semantic`. The dependency direction is:

```text
weavatrix-search-vector -> no graph or semantic dependency
weavatrix-semantic -> weavatrix-search-vector + weavatrix-graph
```

## Dependency policy

- Safe Rust only; `#![forbid(unsafe_code)]`.
- Rust 1.88 minimum.
- No external search/vector engines or native libraries.
- No `usearch`, FAISS, hnswlib, Annoy, or external service/process.
- No general-purpose concurrency dependency; use `std::thread` scoped workers.
- Only the Rust standard library and first-party Weavatrix crates are allowed.
- The index must work on Windows, Linux, and macOS without platform setup.

## MVP capabilities

1. An in-memory approximate nearest-neighbor index implemented in Weavatrix.
2. `f32` dense vectors with cosine distance.
3. Bulk index construction from stable `u64` keys.
4. Top-K candidate lookup for one vector.
5. Batch lookup with bounded parallelism.
6. More than 10,000 vectors with no built-in vector-count ceiling.
7. Memory proportional to vector storage and index degree, never O(n²).
8. Stable ordering for equal distances: lower key wins.
9. Explicit errors for invalid dimensions, non-finite values, duplicate keys,
   zero worker counts, capacity overflow, and allocation/index failures.
10. An exact brute-force oracle for tests and benchmarks.

HNSW is the preferred MVP algorithm because it can meet the latency target
without quadratic comparisons. Its level generation must accept a fixed seed.
Construction may use multiple independently seeded/ordered replicas when that
is necessary to meet recall without serializing the entire build.

## Proposed public API

Names may change, but the ownership boundary must remain equivalent:

```rust
pub enum DistanceMetric {
    Cosine,
}

pub struct IndexConfig {
    pub dimensions: usize,
    pub metric: DistanceMetric,
    pub connectivity: usize,
    pub expansion_build: usize,
    pub expansion_query: usize,
    pub replicas: usize,
    pub build_threads: usize,
    pub query_threads: usize,
    pub seed: u64,
}

pub struct SearchHit {
    pub key: u64,
    pub distance: f32,
}

pub struct VectorIndex { /* private representation */ }

impl VectorIndex {
    pub fn build(
        config: IndexConfig,
        vectors: &[(u64, &[f32])],
    ) -> Result<Self, SearchError>;

    pub fn search(
        &self,
        query: &[f32],
        count: usize,
    ) -> Result<Vec<SearchHit>, SearchError>;

    pub fn search_batch(
        &self,
        queries: &[&[f32]],
        count: usize,
    ) -> Result<Vec<Vec<SearchHit>>, SearchError>;
}
```

The built index should be immutable and `Send + Sync`. Incremental mutation can
be added later without weakening the immutable API.

## Semantic integration contract

`weavatrix-semantic` remains responsible for:

- graph-node and embedding-model validation;
- mapping sorted `NodeId` values to stable search keys;
- requesting an oversampled candidate pool;
- recomputing the final cosine score from original vectors;
- applying the model-specific similarity threshold;
- stable semantic top-K ordering;
- `Mutual`, `Union`, or directional selection;
- applying SEO eligibility after candidate discovery;
- emitting evidence-carrying directed graph edges;
- provenance, confidence, ranks, model, dimensions, and linker version;
- marking candidate coverage as approximate.

`weavatrix-search` must not know about `Graph`, `NodeId`, embedding model names,
thresholds, mutual/union semantics, evidence, or SEO.

## Acceptance gates

Benchmark corpus:

- deterministic clustered synthetic vectors;
- 10,000 vectors;
- 384 dimensions;
- cosine metric;
- top-8 candidates;
- release build;
- one warm-up followed by three measured runs;
- medians reported separately for build, query, and total.

Required results on the reference Intel Core Ultra 7 255U Windows machine:

- `weavatrix-search` build plus all 10,000 queries: at most 3 seconds;
- full `weavatrix-semantic` linking pipeline: at most 5 seconds;
- recall@8 against the exact oracle: at least 99.9%;
- final unordered semantic-pair recall: at least 99.9%;
- the fixed reference corpus should retain 100% semantic-pair recall;
- no hard failure or fixed limit above 5,000 vectors;
- memory at 10,000 × 384 must remain below 256 MiB.

The local 2026-07-27 implementations pass these gates:

- Vector build plus 10,000 queries: 582.331 ms median;
- Vector full-oracle recall@8: 99.9713%;
- Vector retained allocation estimate: 17.958 MiB;
- full Semantic pipeline: 1,085.796 ms median;
- final exact semantic-pair recall: 99.9919%;
- directional SEO Semantic pipeline: 1,539.171 ms median;
- SEO policy violations: zero;
- exact directed-edge recall on the 1,500-page oracle: 100%.

Every benchmark result must include:

- CPU, OS, Rust version, dimensions, vector count, top-K, and index settings;
- warm-up and run count;
- median build/query/total times;
- recall definition and exact-oracle method;
- retained candidate and semantic-pair counts.

## Correctness and reliability tests

- empty index and one-vector index;
- top-K greater than available vectors;
- duplicate and sparse keys;
- equal-distance stable tie ordering;
- input-order independence when a fixed seed is used;
- dimension mismatch;
- NaN, infinity, and zero vector;
- concurrent searches on the same immutable index;
- worker panic/error propagation without deadlock;
- capacity and integer-overflow boundaries;
- exact-oracle equivalence on small randomized corpora;
- recall regression test on the fixed 10,000-vector corpus;
- Windows, Linux, and macOS CI;
- MSRV 1.88, rustfmt, clippy with warnings denied, rustdoc warnings denied.

## Not required for the first version

- text, regex, or repository-source search;
- embedding generation;
- graph construction;
- persistence or memory-mapped indexes;
- incremental deletion/update;
- metadata filtering;
- distributed or hosted search;
- quantization other than `f32`.

These can be added only after the in-memory candidate engine meets the semantic
latency, recall, portability, and dependency gates above.
