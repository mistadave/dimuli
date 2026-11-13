//! Predefined layouts per image count.

use crate::MAX_IMAGES;
use crate::layout::{LayoutNode, SplitDir};

/// Maximum number of cells in one row/column of a grid template.
const MAX_PER_LINE: usize = 4;
/// Maximum number of rows/columns of a grid template.
const MAX_LINES: usize = 4;

/// A layout for a specific number of images. Cells are numbered `0..n` in pre-order.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    name: String,
    layout: LayoutNode,
}

impl Template {
    fn new(name: impl Into<String>, mut layout: LayoutNode) -> Self {
        layout.renumber();
        Self {
            name: name.into(),
            layout,
        }
    }

    /// Stable identifier, e.g. `rows-2-3` or `big-left-3`.
    ///
    /// `rows-a-b` means rows (top to bottom) with `a` and `b` cells side by side;
    /// `cols-a-b` means columns (left to right) with `a` and `b` cells stacked.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The layout; cells are numbered `0..n` in pre-order.
    pub fn layout(&self) -> &LayoutNode {
        &self.layout
    }

    /// Number of cells.
    pub fn cell_count(&self) -> usize {
        self.layout.cell_count()
    }
}

/// All templates for `count` images, the recommended default first.
/// Returns an empty list for `0` or more than [`MAX_IMAGES`] images.
pub fn templates_for(count: usize) -> Vec<Template> {
    let mut out: Vec<Template> = Vec::new();
    if count == 0 || count > MAX_IMAGES {
        return out;
    }
    if count == 1 {
        out.push(Template::new("single", LayoutNode::cell(0)));
        return out;
    }

    let mut push = |template: Template| {
        if !out.iter().any(|t| t.layout == template.layout) {
            out.push(template);
        }
    };

    for shape in grid_shapes(count) {
        let suffix = shape
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("-");
        push(Template::new(
            format!("rows-{suffix}"),
            lines(SplitDir::TopBottom, &shape),
        ));
        push(Template::new(
            format!("cols-{suffix}"),
            lines(SplitDir::LeftRight, &shape),
        ));
    }

    if (3..=MAX_PER_LINE + 1).contains(&count) {
        let rest = || even(SplitDir::TopBottom, vec![LayoutNode::cell(0); count - 1]);
        push(Template::new(
            format!("big-left-{count}"),
            LayoutNode::split(SplitDir::LeftRight, 0.6, LayoutNode::cell(0), rest()),
        ));
        let rest = || even(SplitDir::LeftRight, vec![LayoutNode::cell(0); count - 1]);
        push(Template::new(
            format!("big-top-{count}"),
            LayoutNode::split(SplitDir::TopBottom, 0.6, LayoutNode::cell(0), rest()),
        ));
    }
    out
}

/// Distributions of `count` cells over lines (rows or columns), most balanced first.
/// E.g. 5 → `[2, 3]`, `[3, 2]`, `[1, 2, 2]`, ...
fn grid_shapes(count: usize) -> Vec<Vec<usize>> {
    let ideal = (count as f32).sqrt();
    let mut line_counts: Vec<usize> = (1..=MAX_LINES.min(count))
        .filter(|&lines| count.div_ceil(lines) <= MAX_PER_LINE)
        .collect();
    line_counts.sort_by(|a, b| {
        let da = (*a as f32 - ideal).abs();
        let db = (*b as f32 - ideal).abs();
        da.total_cmp(&db).then(a.cmp(b))
    });

    let mut shapes = Vec::new();
    for lines in line_counts {
        let base = count / lines;
        let extra = count % lines;
        let ascending: Vec<usize> = (0..lines)
            .map(|i| if i >= lines - extra { base + 1 } else { base })
            .collect();
        let mut descending = ascending.clone();
        descending.reverse();
        shapes.push(ascending.clone());
        if descending != ascending {
            shapes.push(descending);
        }
    }
    shapes
}

/// Lines along `dir`, each containing `shape[i]` equal cells in the perpendicular direction.
fn lines(dir: SplitDir, shape: &[usize]) -> LayoutNode {
    let cross = match dir {
        SplitDir::TopBottom => SplitDir::LeftRight,
        SplitDir::LeftRight => SplitDir::TopBottom,
    };
    let children = shape
        .iter()
        .map(|&n| even(cross, vec![LayoutNode::cell(0); n]))
        .collect();
    even(dir, children)
}

/// Equally sized children along `dir`, built as a right-leaning chain of splits with ratios `1/k`.
fn even(dir: SplitDir, children: Vec<LayoutNode>) -> LayoutNode {
    let mut iter = children.into_iter().rev();
    let Some(mut acc) = iter.next() else {
        return LayoutNode::cell(0);
    };
    let mut k = 1;
    for child in iter {
        k += 1;
        acc = LayoutNode::split(dir, 1.0 / k as f32, child, acc);
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::CellId;

    #[test]
    fn no_templates_for_zero_or_too_many_images() {
        assert!(templates_for(0).is_empty());
        assert!(templates_for(MAX_IMAGES + 1).is_empty());
    }

    #[test]
    fn every_supported_count_has_templates_with_matching_cells() {
        for count in 1..=MAX_IMAGES {
            let templates = templates_for(count);
            assert!(!templates.is_empty(), "no template for {count}");
            for t in &templates {
                assert_eq!(t.cell_count(), count, "{}", t.name());
                let expected: Vec<CellId> = (0..count as u32).map(CellId).collect();
                assert_eq!(t.layout().cell_ids(), expected, "{}", t.name());
            }
        }
    }

    #[test]
    fn templates_are_unique() {
        for count in 1..=MAX_IMAGES {
            let templates = templates_for(count);
            for (i, a) in templates.iter().enumerate() {
                for b in templates.iter().skip(i + 1) {
                    assert_ne!(a.layout(), b.layout(), "{} == {}", a.name(), b.name());
                    assert_ne!(a.name(), b.name());
                }
            }
        }
    }

    #[test]
    fn default_template_is_balanced_grid() {
        assert_eq!(templates_for(4)[0].name(), "rows-2-2");
        assert_eq!(templates_for(9)[0].name(), "rows-3-3-3");
        // One row with two cells side by side.
        assert_eq!(templates_for(2)[0].name(), "rows-2");
    }

    #[test]
    fn grid_cells_are_equal_sized() {
        let template = &templates_for(9)[0];
        let rects = template
            .layout()
            .cell_rects(Rect::new(0.0, 0.0, 1.0, 1.0), 0.03);
        let first = rects[0].1;
        for (_, r) in &rects {
            assert!((r.width - first.width).abs() < 1e-5);
            assert!((r.height - first.height).abs() < 1e-5);
        }
    }

    #[test]
    fn grid_shapes_respect_line_limits() {
        for count in 1..=MAX_IMAGES {
            for shape in grid_shapes(count) {
                assert_eq!(shape.iter().sum::<usize>(), count);
                assert!(shape.len() <= MAX_LINES);
                assert!(shape.iter().all(|&n| (1..=MAX_PER_LINE).contains(&n)));
            }
        }
    }
}
