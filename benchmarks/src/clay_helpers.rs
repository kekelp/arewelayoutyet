use std::time::{Duration, Instant};

use clay_layout::id::Id;
use clay_layout::layout::{LayoutDirection, Padding, Sizing};
use clay_layout::{Clay, ClayLayoutScope, Declaration};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

struct ClayNode {
    declaration: Declaration<'static, (), ()>,
    children: Vec<ClayNode>,
}

fn element_id(index: u32) -> Id {
    Id {
        id: unsafe { clay_layout::Clay_GetElementIdWithIndex("pangui".into(), index) },
    }
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
            value: 1.0,
        }) => Sizing::Grow(lower, upper),
        Some(BenchmarkSize {
            mode: SizeMode::Percentage,
            value,
        }) if minimum.is_none() && maximum.is_none() && (0.0..=1.0).contains(value) => {
            Sizing::Percent(*value)
        }
        Some(BenchmarkSize {
            mode: SizeMode::Ratio,
            value,
        }) if minimum.is_none() && maximum.is_none() && value.is_finite() && *value > 0.0 => {
            Sizing::Fit(lower, upper)
        }
        Some(BenchmarkSize {
            mode: SizeMode::Fit,
            ..
        })
        | None => Sizing::Fit(lower, upper),
        _ => return None,
    })
}

fn spacing(value: Option<f32>) -> Option<u16> {
    let value = value.unwrap_or(0.0);
    (value.is_finite() && value >= 0.0 && value <= u16::MAX as f32 && value.fract() == 0.0)
        .then_some(value as u16)
}

fn translate(template: &LayoutNode, count: &mut u32) -> Option<ClayNode> {
    if template.wrap == Some(WrapMode::Auto)
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
    let mut declaration = Declaration::new();
    declaration.id(element_id(*count));
    *count = count.checked_add(1)?;
    declaration
        .layout()
        .width(sizing(
            &template.width,
            &template.min_width,
            &template.max_width,
        )?)
        .height(sizing(
            &template.height,
            &template.min_height,
            &template.max_height,
        )?)
        .padding(Padding::new(
            spacing(template.padding_left)?,
            spacing(template.padding_right)?,
            spacing(template.padding_top)?,
            spacing(template.padding_bottom)?,
        ))
        .child_gap(spacing(template.gap)?)
        .direction(
            if template.flow_direction == Some(FlowDirection::Horizontal) {
                LayoutDirection::LeftToRight
            } else {
                LayoutDirection::TopToBottom
            },
        );
    match (&template.width, &template.height) {
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
        ) => {
            declaration.aspect_ratio(*value);
        }
        (
            _,
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                value,
            }),
        ) => {
            declaration.aspect_ratio(1.0 / *value);
        }
        _ => {}
    }
    let children = template
        .children
        .iter()
        .map(|child| translate(child, count))
        .collect::<Option<Vec<_>>>()?;
    Some(ClayNode {
        declaration,
        children,
    })
}

fn declare<'clay: 'render, 'render>(
    scope: &mut ClayLayoutScope<'clay, 'render, (), ()>,
    node: &ClayNode,
) {
    scope.with(&node.declaration, |scope| {
        for child in &node.children {
            declare(scope, child);
        }
    });
}

pub struct ClayTree {
    clay: Clay,
    root: ClayNode,
}

impl ClayTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        let mut count = 0;
        let root = translate(template, &mut count)?;
        let capacity = i32::try_from(count.checked_add(16)?.max(8192)).ok()?;
        // Set the global limit before Clay allocates its arena.
        unsafe { clay_layout::Clay_SetMaxElementCount(capacity) };
        let clay = Clay::new((width, height).into());
        Some(Self { clay, root })
    }

    pub fn layout_duration(&mut self) -> Duration {
        let start = Instant::now();
        let mut scope = self.clay.begin::<(), ()>();
        declare(&mut scope, &self.root);
        let _ = std::hint::black_box(scope.end());
        start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use crate::clay_helpers::{element_id, ClayTree};
    use crate::pangui_template::parse_test_case;

    #[test]
    fn translates_supported_workloads_and_computes_geometry() {
        for (template, supported) in [
            (include_str!("../pangui/wide_no_wrap_simple_few.txt"), true),
            (include_str!("../pangui/wide_no_wrap_simple_many.txt"), true),
            (include_str!("../pangui/fit_nesting.txt"), true),
            (include_str!("../pangui/nested_vertical_stack.txt"), true),
            (
                include_str!("../pangui/flex_expand_equal_weights.txt"),
                true,
            ),
            (include_str!("../pangui/flex_expand_weights.txt"), false),
            (
                include_str!("../pangui/expand_with_min_constraint.txt"),
                true,
            ),
            (
                include_str!("../pangui/expand_with_max_constraint.txt"),
                true,
            ),
            (
                include_str!("../pangui/pixels_with_min_expand_constraint.txt"),
                false,
            ),
            (include_str!("../pangui/percentage_and_ratio.txt"), true),
            (include_str!("../pangui/wide_wrapping.txt"), false),
            (
                include_str!("../pangui/perpendicular_expand_with_wrap.txt"),
                false,
            ),
            (include_str!("../pangui/padding_and_margin.txt"), false),
        ] {
            let root = parse_test_case("clay.txt", template)
                .expect("valid template")
                .root_node
                .expect("template root");
            let mut tree = ClayTree::new(&root, 1000.0, 1000.0);
            assert_eq!(tree.is_some(), supported);
            if let Some(tree) = &mut tree {
                tree.layout_duration();
                assert!(tree.clay.bounding_box(element_id(0)).is_some());
            }
        }
        let root = parse_test_case("clay.txt", "# Input\nWidth(Pixels(100)).Height(Pixels(40)).Horizontal() { Width(Expand).Height(Expand) {} Width(Expand).Height(Expand) {} }")
            .expect("valid template").root_node.expect("template root");
        let mut tree = ClayTree::new(&root, 100.0, 40.0).expect("supported layout");
        for _ in 0..2 {
            tree.layout_duration();
            let first = tree.clay.bounding_box(element_id(1)).expect("first child");
            let second = tree.clay.bounding_box(element_id(2)).expect("second child");
            assert_eq!(
                (first.x, first.y, first.width, first.height),
                (0.0, 0.0, 50.0, 40.0)
            );
            assert_eq!(
                (second.x, second.y, second.width, second.height),
                (50.0, 0.0, 50.0, 40.0)
            );
        }
    }
}
