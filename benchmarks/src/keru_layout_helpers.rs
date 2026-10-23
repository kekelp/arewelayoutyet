//! Adapter for keru's layout crate, `keru_layout`, through its example tree.
use keru_layout::example_tree::{ExampleTree, NodeI, ROOT_I};
use keru_layout::{Arrange, Axis, ChildrenLayout, LayoutParams, Pos, Size, TreeAdapter, Xy};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

/// Translates a template size. `Ratio(v)` on the width means `width = v * height`, and on the height `height = v * width`.
fn size(size: &Option<BenchmarkSize>, axis: Axis) -> Option<Size> {
    Some(match size {
        None | Some(BenchmarkSize { mode: SizeMode::Fit, .. }) => Size::FitContent,
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Size::Pixels(*value),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Size::Frac(*value),
        // keru has no weights.
        Some(BenchmarkSize { mode: SizeMode::Expand, value }) if *value == 1.0 => Size::Fill,
        Some(BenchmarkSize { mode: SizeMode::Ratio, value }) => match axis {
            Axis::X => Size::AspectRatio(*value),
            Axis::Y => Size::AspectRatio(1.0 / *value),
        },
        Some(_) => return None,
    })
}

fn bound(bound: &Option<BenchmarkSize>) -> Option<Option<Size>> {
    Some(match bound {
        None => None,
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Some(Size::Pixels(*value)),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Some(Size::Frac(*value)),
        Some(BenchmarkSize { mode: SizeMode::Expand, value }) if *value == 1.0 => Some(Size::Fill),
        Some(_) => return None,
    })
}

/// Translates a template node. Returns `None` if the node uses a feature that keru doesn't have: margins, wrapping, expand weights, or different paddings on the two sides of an axis.
fn params(node: &LayoutNode) -> Option<LayoutParams> {
    let margins = [node.margin_left, node.margin_right, node.margin_top, node.margin_bottom];
    if margins.iter().any(|margin| margin.unwrap_or(0.0) != 0.0) || node.wrap == Some(WrapMode::Auto) {
        return None;
    }
    let (left, right) = (node.padding_left.unwrap_or(0.0), node.padding_right.unwrap_or(0.0));
    let (top, bottom) = (node.padding_top.unwrap_or(0.0), node.padding_bottom.unwrap_or(0.0));
    if left != right || top != bottom {
        return None;
    }
    let axis = match node.flow_direction {
        Some(FlowDirection::Horizontal) => Axis::X,
        Some(FlowDirection::Vertical) | None => Axis::Y,
    };
    Some(LayoutParams {
        size: Xy::new(size(&node.width, Axis::X)?, size(&node.height, Axis::Y)?),
        min_size: Xy::new(bound(&node.min_width)?, bound(&node.min_height)?),
        max_size: Xy::new(bound(&node.max_width)?, bound(&node.max_height)?),
        padding: Xy::new(left, top),
        children_layout: ChildrenLayout::Stack { arrange: Arrange::Start, axis, spacing: node.gap.unwrap_or(0.0) },
        position: Xy::new(Pos::Start, Pos::Start),
        ..LayoutParams::DEFAULT
    })
}

/// Counts the nodes of a template, to give the trees their capacity.
fn count(node: &LayoutNode) -> usize {
    1 + node.children.iter().map(count).sum::<usize>()
}

fn add(tree: &mut ExampleTree, parent: NodeI, node: &LayoutNode) -> Option<()> {
    let i = tree.add(parent, params(node)?);
    for child in &node.children {
        add(tree, i, child)?;
    }
    Some(())
}

pub struct KeruLayoutTree {
    tree: ExampleTree,
}

impl KeruLayoutTree {
    /// The template root is added as the only child of the tree's root node, which always takes the whole window.
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        let mut tree = ExampleTree::with_capacity(count(template) + 1, Xy::new(width, height));
        add(&mut tree, ROOT_I, template)?;
        Some(Self { tree })
    }

    /// The solver rebuilds all of its data on every run, so there is nothing to invalidate.
    pub fn mark_all_dirty(&mut self) {}

    pub fn compute_layout(&mut self) {
        self.tree.solve_layout();
        std::hint::black_box(&self.tree);
    }
}
