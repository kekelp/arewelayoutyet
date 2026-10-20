use std::ops::Bound;

use backer::nodes::{column_spaced_aligned, draw, row_spaced_aligned, space};
use backer::{Align, Area, Layout};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

type BackerLayout = Layout<'static, (), Vec<Area>>;

#[derive(Clone, Copy)]
enum AxisMode {
    Fixed(f32),
    Fit,
    Expand,
    Ratio(f32),
}

struct Axis {
    mode: AxisMode,
    minimum: Option<f32>,
    maximum: Option<f32>,
}

fn bound(size: &Option<BenchmarkSize>) -> Option<Option<f32>> {
    match size {
        Some(BenchmarkSize {
            mode: SizeMode::Pixels,
            value,
        }) if value.is_finite() && *value >= 0.0 => Some(Some(*value)),
        None => Some(None),
        _ => None,
    }
}

impl Axis {
    fn new(
        size: &Option<BenchmarkSize>,
        minimum: &Option<BenchmarkSize>,
        maximum: &Option<BenchmarkSize>,
        main_axis: bool,
    ) -> Option<Self> {
        let minimum = bound(minimum)?;
        let maximum = bound(maximum)?;
        let lower = minimum.unwrap_or(0.0);
        let upper = maximum.unwrap_or(f32::INFINITY);
        if lower > upper {
            return None;
        }
        let mode = match size {
            Some(BenchmarkSize {
                mode: SizeMode::Pixels,
                value,
            }) if value.is_finite() && *value >= 0.0 => AxisMode::Fixed(value.clamp(lower, upper)),
            Some(BenchmarkSize {
                mode: SizeMode::Expand,
                value,
            }) if value.is_finite() && *value > 0.0 && (!main_axis || *value == 1.0) => {
                AxisMode::Expand
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
                AxisMode::Ratio(*value)
            }
            Some(BenchmarkSize {
                mode: SizeMode::Fit,
                ..
            })
            | None => AxisMode::Fit,
            _ => return None,
        };
        Some(Self {
            mode,
            minimum,
            maximum,
        })
    }

    fn apply(
        &self,
        layout: BackerLayout,
        horizontal: bool,
        leaf: bool,
        padding: f32,
    ) -> BackerLayout {
        let minimum = if matches!(self.mode, AxisMode::Expand) {
            Some(self.minimum.unwrap_or(0.0))
        } else {
            self.minimum
        };
        let range = (
            minimum.map_or(Bound::Unbounded, Bound::Included),
            self.maximum.map_or(Bound::Unbounded, Bound::Included),
        );
        let layout = if horizontal {
            layout.width_range(range)
        } else {
            layout.height_range(range)
        };
        match self.mode {
            AxisMode::Fixed(value) => {
                if horizontal {
                    layout.width(value)
                } else {
                    layout.height(value)
                }
            }
            AxisMode::Fit if leaf => {
                let value = padding.clamp(
                    self.minimum.unwrap_or(0.0),
                    self.maximum.unwrap_or(f32::INFINITY),
                );
                if horizontal {
                    layout.width(value)
                } else {
                    layout.height(value)
                }
            }
            AxisMode::Fit => layout,
            AxisMode::Expand => {
                if horizontal {
                    layout.expand_x()
                } else {
                    layout.expand_y()
                }
            }
            AxisMode::Ratio(value) => {
                if horizontal {
                    layout.aspect_width(value)
                } else {
                    layout.aspect_height(1.0 / value)
                }
            }
        }
    }

    fn valid(&self, value: f32) -> bool {
        let tolerance = value.abs().max(1.0) * 0.0001;
        value.is_finite()
            && value >= 0.0
            && value + tolerance >= self.minimum.unwrap_or(0.0)
            && value - tolerance <= self.maximum.unwrap_or(f32::INFINITY)
            && match self.mode {
                AxisMode::Fixed(expected) => (value - expected).abs() <= tolerance,
                _ => true,
            }
    }
}

struct Spec {
    width: Axis,
    height: Axis,
    horizontal: bool,
    gap: f32,
    padding: [f32; 4],
    children: Vec<Spec>,
}

fn spacing(value: Option<f32>) -> Option<f32> {
    let value = value.unwrap_or(0.0);
    (value.is_finite() && value >= 0.0).then_some(value)
}

impl Spec {
    fn new(template: &LayoutNode, parent_is_row: bool) -> Option<Self> {
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
        let width = Axis::new(
            &template.width,
            &template.min_width,
            &template.max_width,
            parent_is_row,
        )?;
        let height = Axis::new(
            &template.height,
            &template.min_height,
            &template.max_height,
            !parent_is_row,
        )?;
        if matches!(
            (width.mode, height.mode),
            (AxisMode::Ratio(_), AxisMode::Ratio(_))
        ) {
            return None;
        }
        let horizontal = template.flow_direction == Some(FlowDirection::Horizontal);
        let children = template
            .children
            .iter()
            .map(|child| Self::new(child, horizontal))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            width,
            height,
            horizontal,
            gap: spacing(template.gap)?,
            padding: [
                spacing(template.padding_left)?,
                spacing(template.padding_right)?,
                spacing(template.padding_top)?,
                spacing(template.padding_bottom)?,
            ],
            children,
        })
    }

    fn build(&self, observe: bool) -> BackerLayout {
        let leaf = self.children.is_empty();
        let [left, right, top, bottom] = self.padding;
        let mut layout = if leaf {
            if observe {
                draw(move |area: Area, output: &mut Vec<Area>| {
                    output.push(Area::new(
                        area.x - left,
                        area.y - top,
                        area.width + left + right,
                        area.height + top + bottom,
                    ));
                    Vec::new()
                })
            } else {
                space()
            }
        } else {
            let children = self
                .children
                .iter()
                .map(|child| child.build(observe))
                .collect();
            if self.horizontal {
                row_spaced_aligned(self.gap, Align::TopLeading, children)
            } else {
                column_spaced_aligned(self.gap, Align::TopLeading, children)
            }
        };
        if left == right && left == top && left == bottom {
            if left != 0.0 {
                layout = layout.pad(left);
            }
        } else {
            if left != 0.0 {
                layout = layout.pad_leading(left);
            }
            if right != 0.0 {
                layout = layout.pad_trailing(right);
            }
            if top != 0.0 {
                layout = layout.pad_top(top);
            }
            if bottom != 0.0 {
                layout = layout.pad_bottom(bottom);
            }
        }
        layout = self.width.apply(layout, true, leaf, left + right);
        layout = self.height.apply(layout, false, leaf, top + bottom);
        layout.align(Align::TopLeading)
    }

    fn valid_leaves(&self, areas: &mut impl Iterator<Item = Area>) -> bool {
        if self.children.is_empty() {
            let Some(area) = areas.next() else {
                return false;
            };
            area.x.is_finite()
                && area.y.is_finite()
                && self.width.valid(area.width)
                && self.height.valid(area.height)
        } else {
            self.children.iter().all(|child| child.valid_leaves(areas))
        }
    }
}

pub struct BackerTree {
    spec: Spec,
    layout: BackerLayout,
    area: Area,
    state: Vec<Area>,
}

impl BackerTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        let spec = Spec::new(template, false)?;
        let layout = spec.build(false);
        let tree = Self {
            spec,
            layout,
            area: Area::new(0.0, 0.0, width, height),
            state: Vec::new(),
        };
        let mut areas = tree.leaf_areas().into_iter();
        if !tree.spec.valid_leaves(&mut areas) || areas.next().is_some() {
            return None;
        }
        Some(tree)
    }

    pub fn leaf_areas(&self) -> Vec<Area> {
        let mut layout = self.spec.build(true);
        let mut areas = Vec::new();
        let _ = layout.draw(self.area, &mut areas);
        areas
    }

    pub fn mark_all_dirty(&mut self) {
        self.layout = self.spec.build(false);
    }

    pub fn compute_layout(&mut self) {
        // This public entrypoint runs both complete resolve/allocation rounds.
        std::hint::black_box(self.layout.min_width(self.area, &mut self.state));
        std::hint::black_box(&self.layout);
    }
}
