# Semantic SEO competitor benchmark

Snapshot: 2026-07-27.

This is a capability benchmark against publicly documented product behavior,
plus a reproducible local correctness/performance benchmark. Commercial tools
were not executed on a shared private corpus, so the table does not claim
comparative latency, recall, or ranking lift.

## Capability comparison

| Capability | Weavatrix Semantic | Public competitor evidence |
| --- | --- | --- |
| Directional source-to-target suggestions | `SelectionMode::Directed` | Link Whisper exposes source sentence, anchor, and target; MarketMuse Connect exposes anchors and URLs |
| Semantic relevance | Exact page cosine; optional first-party ANN candidates with exact re-score | InLinks documents entity/knowledge-graph analysis; MarketMuse documents topic models |
| Existing-link suppression | Explicit targets or caller-selected graph link evidence | Internal-link products distinguish existing and suggested links |
| Indexability/canonical eligibility | Fail-closed `SeoPage` source/target and canonical policy | Screaming Frog Link Score excludes redirects and canonicalized pages |
| Same-site and language policy | Strict by default; explicit cross-language opt-in | Site/language boundaries are handled by SEO crawlers and suites |
| Per-source recommendation budget | Deterministic `top_k` | Competitors expose recommendation lists/workflows |
| Cornerstone/orphan prioritization | Preserved as separate target evidence | Yoast prioritizes cornerstone and reports orphaned content; Link Whisper reports orphan pages |
| Anchor/context placement | Exact ranking of caller-extracted anchor text, context, and locator | Link Whisper, InLinks, and MarketMuse expose anchor/context recommendations |
| Evidence and reproducibility | Model, exact score, rank, dimensions, policy, backend, provenance, confidence, version | No equivalent deterministic graph-evidence contract was found in reviewed public docs |
| Automatic HTML/CMS mutation | Deliberately downstream | Several commercial plugins provide insertion/automation |
| Embedding/entity extraction | Deliberately upstream | InLinks includes entity analysis; commercial suites bundle content analysis |

Sources:

- Google Search Central:
  <https://developers.google.com/search/docs/crawling-indexing/links-crawlable>
- Yoast cornerstone and site-structure workflows:
  <https://yoast.com/what-is-cornerstone-content/> and
  <https://yoast.com/improve-your-sites-structure-in-4-simple-steps/>
- Link Whisper: <https://linkwhisper.com/>
- InLinks: <https://inlinks.net/en/how-it-works>
- MarketMuse Connect:
  <https://help.marketmuse.com/support/solutions/articles/80001167936-connect>
- Screaming Frog Link Score:
  <https://www.screamingfrog.co.uk/seo-spider/tutorials/link-score/>

## Local executable benchmark

Reference machine:

- Intel Core Ultra 7 255U;
- Windows 11 Enterprise 10.0.26200;
- Rust 1.97.1, `x86_64-pc-windows-gnu`;
- release build, one warm-up, three measured runs;
- 10,000 pages × 384 dimensions;
- four sites, two languages, canonical duplicates, ineligible sources and
  targets, existing links, cornerstone/orphan/priority evidence;
- directional top-8;
- first-party `weavatrix-search-vector` 0.2.0 candidates, pool multiplier 2.

Observed result:

| Gate | Result |
| --- | ---: |
| Median full vector + SEO semantic pipeline | 1,539.171 ms |
| Directed edges | 79,086 |
| Policy-forbidden edges | 0 |
| Sources above top-8 | 0 |
| Exact-oracle pages | 1,500 |
| Exact directed edges | 11,864 |
| Approximate directed edges | 11,864 |
| Exact directed-edge recall | 100% |

Run:

```console
cargo run --release --locked --features vector-search --example seo_benchmark -- 10000
```

The exact oracle uses the same semantic and SEO policy with exhaustive
eligible-pair cosine comparison. Recall is the intersection of emitted
directed source-target edges divided by exact directed edges.

## Remaining product-level gaps

The semantic crate no longer lacks directional eligibility, evidence, or
anchor placement. A complete Weavatrix SEO product still needs:

- a web crawler or exported-site adapter;
- canonical/indexability/status/language extraction;
- content fragment extraction and an embedding provider;
- authority/PageRank and business-priority inputs;
- review UI/export and optional CMS/HTML mutation;
- evaluation on labeled real sites for recommendation precision and editorial
  acceptance.

These are integration layers or data sources, not vector-search features and
not reasons to move crawling or HTML mutation into the semantic crate.
