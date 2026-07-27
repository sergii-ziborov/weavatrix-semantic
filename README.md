# Weavatrix Semantic

[![CI](https://github.com/sergii-ziborov/weavatrix-semantic/actions/workflows/ci.yml/badge.svg)](https://github.com/sergii-ziborov/weavatrix-semantic/actions/workflows/ci.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/sergii-ziborov/weavatrix-semantic/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue.svg)](Cargo.toml)

`weavatrix-semantic` turns embedding vectors into deterministic,
evidence-carrying relations for
[`weavatrix-graph`](https://crates.io/crates/weavatrix-graph).

It deliberately does not crawl websites, read files, extract text, mutate
HTML, or run an embedding model. Those are source-specific boundaries. This
crate validates already-produced vectors, performs exact cosine top-K
selection, applies explicit link policy, ranks extracted anchor placements,
and emits `semantic_similarity` evidence with model, score, dimension,
selection, rank, linker version, and confidence metadata.

## Why a separate crate?

`weavatrix-graph` owns graph integrity, serialization, and graph algorithms.
Semantic similarity is model-dependent inference, so it belongs in a layer
above the graph core. A local-content pipeline can compose:

```text
weavatrix-scan or crawler -> content/canonical/link extractor
                          -> embedding provider
                          -> weavatrix-semantic
                          -> weavatrix-graph / SEO review / HTML writer
```

A live-site pipeline replaces `weavatrix-scan` with an HTTP/browser crawler and
uses the same linker.

## SEO internal-link recommendations

`SeoLinkPolicy` turns general similarity into directional, reviewable internal
link recommendations. It fails closed on missing page profiles and suppresses:

- ineligible sources and targets such as redirects, canonicalized pages, or
  `noindex` pages, as classified by the caller;
- cross-site links;
- cross-language links unless explicitly enabled;
- links between pages with the same canonical content identity;
- already-existing source-to-target links, including links imported from graph
  evidence.

It carries caller-computed `target_cornerstone`, `target_orphan`, and
`target_priority` evidence without hiding those signals inside the cosine
score:

```rust,no_run
use weavatrix_graph::NodeId;
use weavatrix_semantic::{
    LinkConfig, SelectionMode, SemanticLinker, SeoLinkPolicy, SeoPage,
};

# let graph = weavatrix_graph::GraphBuilder::new().build()?;
# let vectors = Vec::new();
let source = SeoPage::new(
    NodeId::new("page:/guide")?,
    "example.com",
    "/guide",
)?.with_language("en")?;
let target = SeoPage::new(
    NodeId::new("page:/reference")?,
    "example.com",
    "/reference",
)?.with_language("en")?.with_cornerstone(true);
let policy = SeoLinkPolicy::new([source, target])?;
let linker = SemanticLinker::new(
    LinkConfig::new("example-embedding-v1", 0.78, 8)
        .with_selection(SelectionMode::Directed),
)?;
let report = linker.link_with_policy(&graph, &vectors, &policy)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`AnchorMatcher` then ranks caller-extracted source fragments against each
target page vector. It returns the exact existing `anchor_text`, surrounding
`context`, opaque source `locator`, and exact cosine score. It does not invent
copy or silently modify HTML. See
[`docs/seo-contract.md`](docs/seo-contract.md) for the complete integration
contract.

## Example

```rust
use weavatrix_graph::{GraphBuilder, Node, NodeId, NodeKind};
use weavatrix_semantic::{LinkConfig, SemanticLinker, SemanticVector};

let mut builder = GraphBuilder::new();
for id in ["page:/rust", "page:/cargo", "page:/recipes"] {
    builder.add_node(Node::new(
        id,
        id,
        NodeKind::custom("page")?,
    )?)?;
}
let graph = builder.build()?;

let vectors = vec![
    SemanticVector::new("page:/rust", vec![1.0, 0.0])?,
    SemanticVector::new("page:/cargo", vec![0.98, 0.10])?,
    SemanticVector::new("page:/recipes", vec![0.0, 1.0])?,
];
let linker = SemanticLinker::new(LinkConfig::new(
    "example-embedding-v1",
    0.90,
    3,
))?;

let report = linker.link(&graph, &vectors)?;
assert_eq!(report.pair_count(), 1);
assert_eq!(report.edges().len(), 2); // similarity is represented both ways

let augmented = linker.relink(&graph, &vectors)?;
assert_eq!(augmented.node("page:/rust").unwrap().id, NodeId::new("page:/rust")?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Fast first-party vector linking

Enable the `vector-search` feature after adding the first-party
`weavatrix-search-vector` crate:

```rust
use weavatrix_semantic::{
    LinkConfig, VectorCandidateConfig, VectorSemanticLinker,
};

# let graph = weavatrix_graph::GraphBuilder::new().build()?;
# let vectors = Vec::new();
let linker = VectorSemanticLinker::new(
    LinkConfig::new("example-embedding-v1", 0.90, 8),
    VectorCandidateConfig::new(384),
)?;
let report = linker.link(&graph, &vectors)?;
assert_eq!(
    report.candidate_backend().as_str(),
    "weavatrix_search_vector"
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Vector Search owns only deterministic HNSW candidate coverage. Semantic still
owns exact candidate rescoring, model thresholds, stable top-K ordering,
mutual/union/directional selection, SEO policy, exact cosine scoring for
emitted edges, and graph evidence. Every edge records
`candidate_backend=weavatrix_search_vector` and `candidate_exact=false`.

The local 2026-07-27 SEO benchmark used 10,000 clustered 384-dimensional page
vectors, directional top-8 selection, explicit site/language/canonical/
indexability/existing-link policy, one warm-up, and three release runs:

| Evidence | Result |
| --- | ---: |
| Median full vector + SEO semantic pipeline | 1,539 ms |
| Emitted directional recommendations | 79,086 |
| Forbidden recommendations | 0 |
| Sources exceeding top-8 | 0 |
| Exact directed-edge recall on 1,500-page oracle | 100% |

The timing includes validation, index construction, all vector queries,
policy filtering, semantic reconciliation, exact emitted-edge scoring, and
edge construction. Reproduce it with:

```console
cargo run --release --locked --features vector-search --example seo_benchmark -- 10000
```

The backend-only semantic-pair benchmark remains available with:

```console
cargo run --release --features vector-search --example vector_benchmark -- 10000
```

## Selection semantics

Cosine similarity itself is symmetric, while a top-K neighborhood and an SEO
recommendation are directed:

- `SelectionMode::Mutual` (default) keeps a pair only when both endpoints chose
  each other and emits every eligible direction;
- `SelectionMode::Union` keeps a pair when either endpoint chose the other and
  emits every eligible direction;
- `SelectionMode::Directed` emits only the source-selected direction, which is
  the appropriate mode for internal-link recommendations.

Every edge records whether and at what rank each endpoint selected the other.
`relink` removes only older edges emitted by `weavatrix-semantic`, preserves
all other graph evidence, and inserts the new semantic snapshot.

## Current boundary

The exact deterministic implementation performs O(n²) cosine comparisons while
retaining only O(n·k) candidates. It has no fixed vector-count limit by default;
applications may set their own safety bound with `LinkConfig::with_max_vectors`
when they need predictable latency. The optional scalable candidate backend
comes from the first-party `weavatrix-search-vector` crate rather than an
external search engine. Its contract and acceptance gates are documented in
[`docs/weavatrix-search-requirements.md`](docs/weavatrix-search-requirements.md).

The crate never treats a semantic similarity as exact source evidence: emitted
edges use `INFERRED` provenance and caller-selected confidence.

The source crawler remains responsible for crawlability, canonical and
indexability classification, language, existing links, and extracted text
locations. Semantic owns similarity, deterministic top-K, directional policy,
placement ranking, and evidence. A renderer or editor remains responsible for
human approval and HTML mutation.
