# SEO integration contract

## Outcome

`weavatrix-semantic` is the decision layer between crawl/content evidence and
an SEO review or page writer. It returns reproducible recommendations; it does
not claim that similarity alone is a ranking factor and does not mutate source
content.

```text
crawler / weavatrix-scan
  -> page identity, status, canonical, language, existing links
  -> content fragments
  -> embedding provider
  -> page SemanticVector + SeoPage + AnchorCandidate
  -> SemanticLinker or VectorSemanticLinker
  -> directed SEO edges
  -> AnchorMatcher
  -> reviewable target + anchor text + context + locator
  -> graph, UI, export, or HTML writer
```

## Required upstream evidence

For every page vector, provide one `SeoPage`:

- stable graph `NodeId`;
- normalized site identity;
- normalized canonical content identity;
- normalized language when known;
- whether the page may be a source;
- whether the page may be a target;
- existing outgoing internal targets;
- optional cornerstone, orphan, and target-priority signals.

The crawler/content layer decides what redirect, canonical, `noindex`,
robots, HTTP status, content type, or tenant boundaries mean. It maps those
facts to source/target eligibility. Semantic fails closed if any vector lacks a
profile.

Existing links can be attached while building a page or imported from any
caller-selected graph edges with
`SeoLinkPolicy::with_existing_links_from_graph`.

## Directed recommendation contract

Use `SelectionMode::Directed`. Every emitted edge:

- is source-to-target, not an assumed symmetric SEO action;
- passes site, language, canonical, eligibility, and existing-link policy;
- respects `top_k` per source;
- carries exact cosine similarity and deterministic source rank;
- records model, dimension, backend, policy, confidence, and provenance;
- exposes target cornerstone/orphan/priority signals separately from
  similarity.

Approximate vector search is candidate discovery only. Semantic recomputes
exact cosine from original vectors before accepting and scoring an edge.

## Anchor placement contract

The content layer extracts candidate phrases or passages and supplies:

- source page;
- opaque stable locator such as a DOM path or source span;
- exact existing anchor text;
- surrounding context;
- a vector for that context from the same model as the page vectors.

`AnchorMatcher` compares source context with the directed target vector and
returns deterministic ranked placements. It does not generate new words,
parse HTML, or insert a link. This keeps source mutation reviewable and avoids
turning generated copy into unexplained graph evidence.

## Downstream acceptance checks

Before applying a recommendation, an SEO consumer should verify:

1. the source snapshot and locator are still current;
2. the target is still canonical, indexable, and crawlable;
3. no equivalent link was added since the graph snapshot;
4. the selected anchor is concise and natural in context;
5. the resulting link is a crawlable HTML link;
6. policy evidence and exact scores are retained for review/audit.

## Explicit non-goals

- crawling HTTP or rendering JavaScript;
- parsing HTML;
- computing embeddings;
- generating anchor copy;
- writing CMS content;
- predicting ranking lift;
- replacing authority/PageRank analysis.

Those capabilities compose around this crate without contaminating semantic
similarity with source-specific or unverifiable claims.
