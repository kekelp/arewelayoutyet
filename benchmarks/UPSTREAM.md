# Benchmark provenance

Copied from [DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/tree/4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6/benches) at commit `4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6`, under the MIT license in `LICENSE`.

The upstream templates and original helper modules are unchanged. The manifest retains only the PanGUI benchmark target, removes its unused Parley dependency, and replaces the local Taffy path with that pinned Git revision. The original package name is retained for imports. Daily runs enable Taffy, Morphorm, Torin, Yoga, Clay, and our experimental ui_layout adapter. The optional Taffy 0.3 adapter is preserved but not enabled.

The local adapters are `src/ui_layout_helpers.rs` and `src/clay_helpers.rs`, with module declarations in `src/lib.rs` and registrations in `benches/pangui.rs`. ui_layout is pinned to [cffdbc4bc0e4f54b9a4466847528fbfa2127648a](https://github.com/098orin/ui_layout-rs/tree/cffdbc4bc0e4f54b9a4466847528fbfa2127648a), rather than the published 0.12.1 release, to include its newer wrapping and grid work. The adapter translates templates to flex styles and skips aspect-ratio templates because there is no general aspect-ratio style property.

ui_layout's intrinsic-layout caches are private and have no public invalidation API. Before each timed iteration, its adapter reconstructs every node outside the timer to clear these caches. This also discards scratch buffers, unlike the tree-reusing adapters. Within-pass caching remains enabled. Treat its results as experimental, not a layout-equivalence guarantee.

Clay uses the `clay-layout` 0.4.0 Rust bindings and their bundled C engine. The adapter pre-translates styles and allocates an arena large enough for the complete workload outside timing. Each timed pass includes `BeginLayout`, native element declaration, and `EndLayout`, including render-command generation. Clay computes intrinsic sizes while closing declared elements, so timing only `EndLayout` would exclude part of its layout work. This differs from the retained-tree adapters, whose tree construction is excluded. No render commands are consumed or drawn. Every pass starts a fresh layout, with no text measurement involved.

Clay maps fixed, percentage, fit, and equal-grow sizing, pixel min/max bounds, padding, gaps, and aspect ratios to native properties. It skips child wrapping, in-flow margins, unequal grow weights, and `Expand` minimum sizes. It also rejects unsupported combinations such as min/max bounds on percentage or ratio dimensions and non-integer padding or gaps. Unsupported workloads remain `/`, without silently substituting a different algorithm.

Templates originate in [PanGUI's layout benchmark article](https://www.pangui.io/blog/05-layout-rework-and-benchmarks/), as attributed upstream.

To update, review and copy the upstream files, update the Git revision here and in Cargo.toml, regenerate Cargo.lock, and check adapter limitations and the feature matrix. Dependencies deliberately do not float nightly.
