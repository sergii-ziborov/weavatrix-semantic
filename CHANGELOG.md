# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 0.2.2 - 2026-07-30

- Normalize anchor, error, linker, and policy modules to one idiomatic Rust
  module-tree form without changing public paths.
- Add a strict modular architecture contract with 300-line files, 100-line
  functions, zero runtime cycles, no baseline, and no exceptions.
- Track `weavatrix-graph` 0.6.2 and the optional
  `weavatrix-search-vector` 0.3.1 candidate backend.
- Document the exact boundary between graph integrity, semantic policy,
  candidate retrieval, source acquisition, and embedding providers.

## 0.2.1 - 2026-07-29

- Prepare the semantic and SEO linking library for immutable crates.io
  publication with pinned CI actions and trusted publishing.

## 0.2.0 - 2026-07-27

- Added deterministic exact cosine top-K semantic linking.
- Added mutual and union pair-selection modes.
- Added directional selection for source-to-target SEO recommendations.
- Added fail-closed `SeoLinkPolicy` filtering for site, language, canonical
  identity, source/target eligibility, and existing internal links.
- Added separate cornerstone, orphan, and target-priority evidence without
  changing exact semantic similarity.
- Added exact anchor/context placement ranking for caller-extracted text.
- Added evidence-carrying `semantic_similarity` graph edges.
- Added idempotent graph relinking and strict vector validation.
- Removed the default 5,000-vector limit.
- Reduced exact-link candidate memory from O(n²) to O(n·k).
- Added an optional first-party `weavatrix-search-vector` candidate backend
  while retaining semantic thresholds, exact emitted-edge scores, and graph
  provenance in this crate.
- Reached a 1.086-second median for the full 10,000 × 384d top-8 semantic
  pipeline at 99.9919% exact semantic-pair recall on the reference corpus.
- Added a reproducible 10,000-page directional SEO benchmark with zero policy
  violations, no top-K budget violations, and 100% exact directed-edge recall
  on a 1,500-page oracle.
