# Are we layout yet?

[arewelayoutyet.mespin.me](https://arewelayoutyet.mespin.me)

A comparison of Rust UI layout libraries:

- [Taffy](https://crates.io/crates/taffy)
- [Morphorm](https://crates.io/crates/morphorm)
- [Torin](https://crates.io/crates/torin)
- [Yoga](https://crates.io/crates/yoga), through Rust bindings to the C++ engine
- [ui-layout (WIP 🚧)](https://github.com/098orin/ui_layout-rs), experimental and pinned to a Git revision

## What is compared?

**Features:** layout models, wrapping, sizing, spacing, constraints, aspect ratios, custom content measurement, and pixel-grid rounding.

**Performance:** daily benchmarks of shared PanGUI workloads, adapted from Taffy's benchmark suite. Results include mean layout times, confidence intervals, and historical runs. Only uncached layout computation is timed, not tree construction.

The engines have different layout semantics. Missing results (`/`) mean the adapter cannot translate that workload or the library was not included in that historical run. Timings do not establish equivalent output or an overall winner.

Text measurement is not currently compared. See [benchmark provenance](benchmarks/UPSTREAM.md) for attribution and adapter details.
