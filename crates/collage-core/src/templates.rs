//! Predefined layouts per image count.

use crate::MAX_IMAGES;
use crate::layout::{LayoutNode, SplitDir};

/// Maximum number of cells in one row/column of a grid template.
const MAX_PER_LINE: usize = 4;
/// Maximum number of rows/columns of a grid template.
const MAX_LINES: usize = 4;
/// Up to this count the original template set is used unchanged (1-9 stay pixel-identical, D-03).
pub(crate) const CLASSIC_MAX_IMAGES: usize = 9;
/// Above [`CLASSIC_MAX_IMAGES`] only this many of the most balanced line counts are offered.
const GRID_LINE_COUNTS: usize = 2;
/// At most this many templates above [`CLASSIC_MAX_IMAGES`], so the template bar fits without scrolling.
const MAX_GRID_TEMPLATES: usize = 6;

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
///
/// Up to [`CLASSIC_MAX_IMAGES`] images this is the original set; above it, at most
/// [`MAX_GRID_TEMPLATES`] even grids from the two most balanced line counts.
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
    if count > CLASSIC_MAX_IMAGES {
        out.truncate(MAX_GRID_TEMPLATES);
    }
    out
}

/// Distributions of `count` cells over lines (rows or columns), most balanced first.
/// E.g. 5 → `[2, 3]`, `[3, 2]`, `[1, 2, 2]`, ...
fn grid_shapes(count: usize) -> Vec<Vec<usize>> {
    let ideal = (count as f32).sqrt();
    let mut line_counts: Vec<usize> = if count > CLASSIC_MAX_IMAGES {
        (1..=count).collect()
    } else {
        (1..=MAX_LINES.min(count))
            .filter(|&lines| count.div_ceil(lines) <= MAX_PER_LINE)
            .collect()
    };
    line_counts.sort_by(|a, b| {
        let da = (*a as f32 - ideal).abs();
        let db = (*b as f32 - ideal).abs();
        da.total_cmp(&db).then(a.cmp(b))
    });
    if count > CLASSIC_MAX_IMAGES {
        line_counts.truncate(GRID_LINE_COUNTS);
    }

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

/// Test-only template with lines of `shape` cells, for layouts with 5+ cells per line.
#[cfg(test)]
pub(crate) fn test_grid(dir: SplitDir, shape: &[usize]) -> Template {
    Template::new("test-grid", lines(dir, shape))
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

    /// Verbatim copy as of 5c2f109, never edit; it is the reference that proves 1-9 are unchanged.
    fn legacy_templates_for(count: usize) -> Vec<Template> {
        let mut out: Vec<Template> = Vec::new();
        if count == 0 || count > 9 {
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

        for shape in legacy_grid_shapes(count) {
            let suffix = shape
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("-");
            push(Template::new(
                format!("rows-{suffix}"),
                legacy_lines(SplitDir::TopBottom, &shape),
            ));
            push(Template::new(
                format!("cols-{suffix}"),
                legacy_lines(SplitDir::LeftRight, &shape),
            ));
        }

        if (3..=4 + 1).contains(&count) {
            let rest = || legacy_even(SplitDir::TopBottom, vec![LayoutNode::cell(0); count - 1]);
            push(Template::new(
                format!("big-left-{count}"),
                LayoutNode::split(SplitDir::LeftRight, 0.6, LayoutNode::cell(0), rest()),
            ));
            let rest = || legacy_even(SplitDir::LeftRight, vec![LayoutNode::cell(0); count - 1]);
            push(Template::new(
                format!("big-top-{count}"),
                LayoutNode::split(SplitDir::TopBottom, 0.6, LayoutNode::cell(0), rest()),
            ));
        }
        out
    }

    /// Verbatim copy as of 5c2f109, never edit; it is the reference that proves 1-9 are unchanged.
    fn legacy_grid_shapes(count: usize) -> Vec<Vec<usize>> {
        let ideal = (count as f32).sqrt();
        let mut line_counts: Vec<usize> = (1..=4usize.min(count))
            .filter(|&lines| count.div_ceil(lines) <= 4)
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

    /// Verbatim copy as of 5c2f109, never edit; it is the reference that proves 1-9 are unchanged.
    fn legacy_lines(dir: SplitDir, shape: &[usize]) -> LayoutNode {
        let cross = match dir {
            SplitDir::TopBottom => SplitDir::LeftRight,
            SplitDir::LeftRight => SplitDir::TopBottom,
        };
        let children = shape
            .iter()
            .map(|&n| legacy_even(cross, vec![LayoutNode::cell(0); n]))
            .collect();
        legacy_even(dir, children)
    }

    /// Verbatim copy as of 5c2f109, never edit; it is the reference that proves 1-9 are unchanged.
    fn legacy_even(dir: SplitDir, children: Vec<LayoutNode>) -> LayoutNode {
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

    #[test]
    fn templates_for_one_to_nine_are_unchanged() {
        for count in 1..=9 {
            assert_eq!(templates_for(count), legacy_templates_for(count), "{count}");
        }
    }

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
        for count in 1..=CLASSIC_MAX_IMAGES {
            for shape in grid_shapes(count) {
                assert_eq!(shape.iter().sum::<usize>(), count);
                assert!(shape.len() <= MAX_LINES);
                assert!(shape.iter().all(|&n| (1..=MAX_PER_LINE).contains(&n)));
            }
        }
    }

    fn names(count: usize) -> Vec<String> {
        templates_for(count)
            .iter()
            .map(|t| t.name().to_string())
            .collect()
    }

    #[test]
    fn many_images_default_templates_are_the_most_balanced() {
        assert_eq!(
            names(10),
            [
                "rows-3-3-4",
                "cols-3-3-4",
                "rows-4-3-3",
                "cols-4-3-3",
                "rows-2-2-3-3",
                "cols-2-2-3-3"
            ]
        );
        assert_eq!(
            names(12),
            ["rows-4-4-4", "cols-4-4-4", "rows-3-3-3-3", "cols-3-3-3-3"]
        );
        assert_eq!(
            names(36),
            [
                "rows-6-6-6-6-6-6",
                "cols-6-6-6-6-6-6",
                "rows-7-7-7-7-8",
                "cols-7-7-7-7-8",
                "rows-8-7-7-7-7",
                "cols-8-7-7-7-7"
            ]
        );
        assert_eq!(names(16)[0], "rows-4-4-4-4");
        assert_eq!(names(25)[0], "rows-5-5-5-5-5");
        assert!(templates_for(MAX_IMAGES + 1).is_empty());
    }

    #[test]
    fn many_images_get_four_to_six_balanced_grids() {
        for count in (CLASSIC_MAX_IMAGES + 1)..=MAX_IMAGES {
            let templates = templates_for(count);
            assert!(
                (4..=6).contains(&templates.len()),
                "{count}: {} templates",
                templates.len()
            );
            for (i, t) in templates.iter().enumerate() {
                let name = t.name();
                assert!(
                    name.starts_with("rows-") || name.starts_with("cols-"),
                    "{count}: {name}"
                );
                assert_eq!(t.cell_count(), count, "{count}: {name}");
                let mut ids = Vec::new();
                collect_ids(t.layout(), &mut ids);
                let expected: Vec<u32> = (0..count as u32).collect();
                assert_eq!(ids, expected, "{count}: {name} not numbered in pre-order");
                for other in templates.iter().skip(i + 1) {
                    assert_ne!(name, other.name(), "{count}: duplicate name");
                    assert!(t.layout() != other.layout(), "{count}: duplicate layout");
                }
            }
        }
    }

    fn collect_ids(node: &LayoutNode, out: &mut Vec<u32>) {
        match node {
            LayoutNode::Cell(id) => out.push(id.0),
            LayoutNode::Split(s) => {
                collect_ids(s.first(), out);
                collect_ids(s.second(), out);
            }
        }
    }
}
