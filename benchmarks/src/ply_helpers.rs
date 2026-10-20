use std::time::{Duration, Instant};

use ply_engine::engine::{
    ElementDeclaration, LayoutConfig, PaddingConfig, PlyContext, SizingConfig,
};
use ply_engine::id::Id;
use ply_engine::layout::{LayoutDirection, Sizing};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

struct PlyNode {
    id: Id,
    declaration: ElementDeclaration<()>,
    children: Vec<PlyNode>,
}

fn bound(size: &Option<BenchmarkSize>, fallback: f32) -> Option<f32> {
    match size {
        Some(BenchmarkSize {
            mode: SizeMode::Pixels,
            value,
        }) if value.is_finite() && *value >= 0.0 => Some(*value),
        None => Some(fallback),
        _ => None,
    }
}

fn sizing(
    size: &Option<BenchmarkSize>,
    minimum: &Option<BenchmarkSize>,
    maximum: &Option<BenchmarkSize>,
) -> Option<Sizing> {
    let lower = bound(minimum, 0.0)?;
    let upper = bound(maximum, f32::MAX)?;
    if lower > upper {
        return None;
    }
    Some(match size {
        Some(BenchmarkSize {
            mode: SizeMode::Pixels,
            value,
        }) if value.is_finite() && *value >= 0.0 => Sizing::Fixed(value.clamp(lower, upper)),
        Some(BenchmarkSize {
            mode: SizeMode::Expand,
            value,
        }) if value.is_finite() && *value > 0.0 && upper > 0.0 => {
            Sizing::Grow(lower, upper, *value)
        }
        Some(BenchmarkSize {
            mode: SizeMode::Percentage,
            value,
        }) if minimum.is_none() && maximum.is_none() && (0.0..=1.0).contains(value) => {
            Sizing::Percent(*value)
        }
        Some(BenchmarkSize {
            mode: SizeMode::Ratio,
            value,
        }) if minimum.is_none()
            && maximum.is_none()
            && value.is_finite()
            && *value > 0.0
            && (1.0 / *value).is_finite() =>
        {
            Sizing::Fit(lower, upper)
        }
        Some(BenchmarkSize {
            mode: SizeMode::Fit,
            ..
        })
        | None
            if upper > 0.0 =>
        {
            Sizing::Fit(lower, upper)
        }
        _ => return None,
    })
}

fn spacing(value: Option<f32>) -> Option<u16> {
    let value = value.unwrap_or(0.0);
    (value.is_finite() && value >= 0.0 && value <= u16::MAX as f32 && value.fract() == 0.0)
        .then_some(value as u16)
}

fn translate(template: &LayoutNode, count: &mut u32) -> Option<PlyNode> {
    if template.children.len() > u16::MAX as usize
        || [
            template.margin_left,
            template.margin_right,
            template.margin_top,
            template.margin_bottom,
        ]
        .iter()
        .any(|margin| margin.is_some_and(|value| value != 0.0))
    {
        return None;
    }
    let id = Id::new_index("pangui", *count);
    *count = count.checked_add(1)?;
    let aspect_ratio = match (&template.width, &template.height) {
        (
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                ..
            }),
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                ..
            }),
        ) => return None,
        (
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                value,
            }),
            _,
        ) => *value,
        (
            _,
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                value,
            }),
        ) => 1.0 / *value,
        _ => 0.0,
    };
    let gap = spacing(template.gap)?;
    let declaration = ElementDeclaration {
        layout: LayoutConfig {
            sizing: SizingConfig {
                width: sizing(&template.width, &template.min_width, &template.max_width)?.into(),
                height: sizing(&template.height, &template.min_height, &template.max_height)?
                    .into(),
            },
            padding: PaddingConfig {
                left: spacing(template.padding_left)?,
                right: spacing(template.padding_right)?,
                top: spacing(template.padding_top)?,
                bottom: spacing(template.padding_bottom)?,
            },
            child_gap: gap,
            wrap: template.wrap == Some(WrapMode::Auto),
            wrap_gap: gap,
            layout_direction: if template.flow_direction == Some(FlowDirection::Horizontal) {
                LayoutDirection::LeftToRight
            } else {
                LayoutDirection::TopToBottom
            },
            ..Default::default()
        },
        aspect_ratio,
        ..Default::default()
    };
    let children = template
        .children
        .iter()
        .map(|child| translate(child, count))
        .collect::<Option<Vec<_>>>()?;
    Some(PlyNode {
        id,
        declaration,
        children,
    })
}

fn declare(context: &mut PlyContext<()>, node: &PlyNode) {
    context.open_element_with_id(&node.id);
    context.configure_open_element(&node.declaration);
    for child in &node.children {
        declare(context, child);
    }
    context.close_element();
}

fn valid_geometry(context: &PlyContext<()>, node: &PlyNode) -> bool {
    let Some(bounds) = context.get_element_data(node.id.clone()) else {
        return false;
    };
    if ![bounds.x, bounds.y, bounds.width, bounds.height]
        .iter()
        .all(|value| value.is_finite())
        || bounds.width < 0.0
        || bounds.height < 0.0
    {
        return false;
    }
    for (actual, sizing) in [
        (bounds.width, node.declaration.layout.sizing.width),
        (bounds.height, node.declaration.layout.sizing.height),
    ] {
        let tolerance = actual.abs().max(1.0) * 0.0001;
        if actual + tolerance < sizing.min_max.min
            || actual - tolerance > sizing.min_max.max && sizing.min_max.max > 0.0
        {
            return false;
        }
    }
    let ratio = node.declaration.aspect_ratio;
    if ratio > 0.0
        && bounds.height > 0.0
        && (bounds.width / bounds.height - ratio).abs() > ratio * 0.0001
    {
        return false;
    }
    node.children
        .iter()
        .all(|child| valid_geometry(context, child))
}

pub struct PlyTree {
    pub context: PlyContext<()>,
    root: PlyNode,
}

impl PlyTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        let mut count = 0;
        let root = translate(template, &mut count)?;
        let capacity = i32::try_from(count.checked_add(16)?.max(8192)).ok()?;
        let mut context = PlyContext::new((width, height).into());
        context.set_max_element_count(capacity);
        let mut tree = Self { context, root };
        tree.layout_duration();
        if !valid_geometry(&tree.context, &tree.root) {
            return None;
        }
        Some(tree)
    }

    pub fn layout_duration(&mut self) -> Duration {
        let start = Instant::now();
        self.context.begin_layout();
        declare(&mut self.context, &self.root);
        let _ = std::hint::black_box(self.context.end_layout());
        start.elapsed()
    }
}
