use ui_layout::{
    AlignContent, AlignItems, BoxSizing, Display, FlexDirection, FlexWrap, InnerDisplay, ItemStyle,
    JustifyContent, LayoutEngine, LayoutNode as UiNode, Length, LengthOrAuto, OuterDisplay,
    SizeStyle, Spacing, Style,
};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

fn is_mode(size: &Option<BenchmarkSize>, mode: SizeMode) -> bool {
    size.is_some_and(|size| size.mode == mode)
}

fn dimension(size: &Option<BenchmarkSize>) -> Option<LengthOrAuto> {
    Some(match size {
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Length::Px(*value).into(),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => {
            Length::Percent(*value * 100.0).into()
        }
        Some(BenchmarkSize { mode: SizeMode::Ratio, .. }) => return None,
        Some(BenchmarkSize { mode: SizeMode::Fit | SizeMode::Expand, .. }) | None => {
            LengthOrAuto::Auto
        }
    })
}

fn node(template: &LayoutNode, parent_is_row: bool) -> Option<UiNode> {
    let (main_size, main_minimum) = if parent_is_row {
        (&template.width, &template.min_width)
    } else {
        (&template.height, &template.min_height)
    };
    let (flex_grow, flex_shrink, flex_basis) = match (main_size, main_minimum) {
        (Some(BenchmarkSize { mode: SizeMode::Expand, value }), _) => {
            (*value, 1.0, Length::Px(0.0).into())
        }
        (_, Some(BenchmarkSize { mode: SizeMode::Expand, value })) => {
            (*value, 0.0, LengthOrAuto::Auto)
        }
        _ => (0.0, 0.0, LengthOrAuto::Auto),
    };
    let mut min_width = dimension(&template.min_width)?;
    let mut min_height = dimension(&template.min_height)?;
    if is_mode(main_size, SizeMode::Expand) {
        if parent_is_row && template.min_width.is_none() {
            min_width = Length::Px(0.0).into();
        }
        if !parent_is_row && template.min_height.is_none() {
            min_height = Length::Px(0.0).into();
        }
    }
    let is_row = template.flow_direction == Some(FlowDirection::Horizontal);
    let gap = LengthOrAuto::from(Length::Px(template.gap.unwrap_or(0.0)));
    let style = Style {
        display: Display::OutsideInner { outer: OuterDisplay::Block, inner: InnerDisplay::Flex },
        size: SizeStyle {
            width: dimension(&template.width)?,
            height: dimension(&template.height)?,
            min_width,
            min_height,
            max_width: dimension(&template.max_width)?,
            max_height: dimension(&template.max_height)?,
            ..Default::default()
        },
        item_style: ItemStyle { flex_grow, flex_shrink, flex_basis, ..Default::default() },
        box_sizing: BoxSizing::BorderBox,
        flex_direction: if is_row { FlexDirection::Row } else { FlexDirection::Column },
        flex_wrap: if template.wrap == Some(WrapMode::Auto) {
            FlexWrap::Wrap
        } else {
            FlexWrap::NoWrap
        },
        column_gap: gap.clone(),
        row_gap: gap,
        spacing: Spacing {
            padding_left: Length::Px(template.padding_left.unwrap_or(0.0)),
            padding_right: Length::Px(template.padding_right.unwrap_or(0.0)),
            padding_top: Length::Px(template.padding_top.unwrap_or(0.0)),
            padding_bottom: Length::Px(template.padding_bottom.unwrap_or(0.0)),
            margin_left: Length::Px(template.margin_left.unwrap_or(0.0)).into(),
            margin_right: Length::Px(template.margin_right.unwrap_or(0.0)).into(),
            margin_top: Length::Px(template.margin_top.unwrap_or(0.0)).into(),
            margin_bottom: Length::Px(template.margin_bottom.unwrap_or(0.0)).into(),
            ..Default::default()
        },
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Start,
        align_content: AlignContent::Start,
        ..Default::default()
    };
    let children = template.children.iter().map(|child| node(child, is_row)).collect::<Option<Vec<_>>>()?;
    Some(UiNode::with_children(style, children))
}

pub struct UiLayoutTree {
    root: UiNode,
    width: f32,
    height: f32,
}

impl UiLayoutTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        Some(Self { root: node(template, false)?, width, height })
    }

    pub fn mark_all_dirty(&mut self) {
        reset_node(&mut self.root);
    }

    pub fn compute_layout(&mut self) {
        LayoutEngine::layout(&mut self.root, self.width, self.height);
    }
}

// Reconstruct nodes outside timing to discard private intrinsic-layout caches.
fn reset_node(node: &mut UiNode) {
    for child in &mut node.children {
        if let Some(child) = child.node_mut() {
            reset_node(child);
        }
    }
    let children = std::mem::take(&mut node.children);
    *node = UiNode::with_children(node.style.clone(), children);
}
