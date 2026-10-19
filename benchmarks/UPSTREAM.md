# Benchmark provenance

Copied from [DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/tree/4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6/benches) at commit `4e59968eda29ceeb0bbaeb3993c8cd38baa3d0a6`, under the MIT license in `LICENSE`.

The PanGUI benchmark, templates and helper modules are unchanged. The manifest only retains the PanGUI benchmark target, removes its unused Parley dependency, and replaces the local Taffy path with that pinned Git revision. The original package name is retained for imports. The Yoga adapter is enabled in daily runs alongside Taffy, Morphorm, and Torin. The optional Taffy 0.3 adapter is preserved but not enabled.

Templates originate in [PanGUI's layout benchmark article](https://www.pangui.io/blog/05-layout-rework-and-benchmarks/), as attributed upstream.

To update, review and copy the upstream files, update the Git revision here and in Cargo.toml, regenerate Cargo.lock, and check adapter limitations and the feature matrix. Dependencies deliberately do not float nightly.
