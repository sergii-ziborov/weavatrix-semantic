# Weavatrix Semantic

[![CI](https://github.com/sergii-ziborov/weavatrix-semantic/actions/workflows/ci.yml/badge.svg)](https://github.com/sergii-ziborov/weavatrix-semantic/actions/workflows/ci.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/sergii-ziborov/weavatrix-semantic/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue.svg)](Cargo.toml)

`weavatrix-semantic` turns embedding vectors into deterministic,
evidence-carrying relations for
[`weavatrix-graph`](https://crates.io/crates/weavatrix-graph).

It deliberately does not crawl websites, read files, extract text, or run an
embedding model. Those are source-specific boundaries. This crate validates
already-produced vectors, performs exact cosine top-K selection, and emits
`semantic_similarity` edges with model, score, dimension, selection, rank,
linker-version, evidence, and confidence metadata.

## Why a separate crate?

`weavatrix-graph` owns graph integrity, serialization, and graph algorithms.
Semantic similarity is model-dependent inference, so it belongs in a layer
above the graph core. A local-content pipeline can compose:

```text
weavatrix-scan -> content extractor -> embedding provider
                -> weavatrix-semantic -> weavatrix-graph
```

A live-site pipeline replaces `weavatrix-scan` with an HTTP/browser crawler and
uses the same linker.

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

## Selection semantics

Cosine similarity itself is symmetric, while a top-K neighborhood is directed.
The linker therefore selects unordered pairs and emits two graph edges per
pair:

- `SelectionMode::Mutual` (default) keeps a pair only when both endpoints chose
  each other;
- `SelectionMode::Union` keeps a pair when either endpoint chose the other.

Every edge records whether and at what rank each endpoint selected the other.
`relink` removes only older edges emitted by `weavatrix-semantic`, preserves
all other graph evidence, and inserts the new semantic snapshot.

## Current boundary

The first implementation is an exact, deterministic O(n²) baseline with a
configurable input bound (5,000 vectors by default). That is appropriate for
small and medium content sets and as a correctness oracle for a future
approximate-nearest-neighbor backend. The crate never treats a similarity score
as exact evidence: emitted edges use `INFERRED` provenance and caller-selected
confidence.
