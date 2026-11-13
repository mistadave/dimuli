//! Layout tree: the collage area is split recursively into cells.
//!
//! Every [`Split`] divides its area into `first` and `second` along one axis, separated by a gap of
//! `spacing`. Each split is one draggable divider; its `ratio` is the only value dragging changes.
//!
//! Sizes are *gap-aware*: a split first subtracts every gap along its axis (its own and those inside its
//! children) and then distributes the remaining content length by `ratio`. That way a chain of splits with
//! ratios `1/k` yields `k` exactly equal cells, regardless of `spacing`.

use crate::error::CoreError;
use crate::geometry::{Rect, Vec2};

/// Minimum size of a cell along a dragged axis, in collage units.
pub const MIN_CELL_SIZE: f32 = 0.05;

/// Identifies a cell. Cells are numbered `0..n` in pre-order (top-left first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellId(pub u32);

/// Identifies a divider (= a split). Dividers are numbered in pre-order of their splits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DividerId(pub u32);

/// Axis along which a split divides its area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDir {
    /// `first` left, `second` right; the divider is a vertical line.
    LeftRight,
    /// `first` on top, `second` below; the divider is a horizontal line.
    TopBottom,
}

impl SplitDir {
    fn extent(self, rect: &Rect) -> f32 {
        match self {
            SplitDir::LeftRight => rect.width,
            SplitDir::TopBottom => rect.height,
        }
    }

    fn component(self, v: Vec2) -> f32 {
        match self {
            SplitDir::LeftRight => v.x,
            SplitDir::TopBottom => v.y,
        }
    }
}

/// A node of the layout tree.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// A leaf showing one image.
    Cell(CellId),
    /// An inner node dividing its area in two.
    Split(Box<Split>),
}

/// An inner node dividing its area into two parts.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    dir: SplitDir,
    ratio: f32,
    first: LayoutNode,
    second: LayoutNode,
}

impl Split {
    /// Axis of the split.
    pub fn dir(&self) -> SplitDir {
        self.dir
    }

    /// Share of the content length (without gaps) that goes to `first`, in `0.0..=1.0`.
    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    /// Left or top part.
    pub fn first(&self) -> &LayoutNode {
        &self.first
    }

    /// Right or bottom part.
    pub fn second(&self) -> &LayoutNode {
        &self.second
    }

    /// Content length along the split axis: extent minus all gaps along that axis.
    fn content(&self, bounds: &Rect, spacing: f32) -> f32 {
        let gaps = 1 + self.first.gaps(self.dir) + self.second.gaps(self.dir);
        (self.dir.extent(bounds) - gaps as f32 * spacing).max(0.0)
    }

    /// Returns `(first, gap, second)` rectangles.
    fn rects(&self, bounds: Rect, spacing: f32) -> (Rect, Rect, Rect) {
        let content = self.content(&bounds, spacing);
        let first_gaps = self.first.gaps(self.dir) as f32 * spacing;
        let second_gaps = self.second.gaps(self.dir) as f32 * spacing;
        let first_len = content * self.ratio + first_gaps;
        let second_len = content * (1.0 - self.ratio) + second_gaps;
        let b = bounds;
        match self.dir {
            SplitDir::LeftRight => (
                Rect::new(b.x, b.y, first_len, b.height),
                Rect::new(b.x + first_len, b.y, spacing, b.height),
                Rect::new(b.x + first_len + spacing, b.y, second_len, b.height),
            ),
            SplitDir::TopBottom => (
                Rect::new(b.x, b.y, b.width, first_len),
                Rect::new(b.x, b.y + first_len, b.width, spacing),
                Rect::new(b.x, b.y + first_len + spacing, b.width, second_len),
            ),
        }
    }
}

/// A draggable divider between two parts of a split.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Divider {
    /// Id for [`LayoutNode::drag_divider`].
    pub id: DividerId,
    /// Axis of the split this divider belongs to.
    pub dir: SplitDir,
    /// The gap between the two parts. Has zero thickness when `spacing` is 0;
    /// the UI is responsible for a large enough touch area.
    pub rect: Rect,
}

impl LayoutNode {
    /// A leaf cell.
    pub(crate) fn cell(id: u32) -> Self {
        LayoutNode::Cell(CellId(id))
    }

    /// A split node. `ratio` is clamped to `0.0..=1.0`; NaN becomes `0.5`.
    pub(crate) fn split(dir: SplitDir, ratio: f32, first: LayoutNode, second: LayoutNode) -> Self {
        let ratio = if ratio.is_nan() {
            0.5
        } else {
            ratio.clamp(0.0, 1.0)
        };
        LayoutNode::Split(Box::new(Split {
            dir,
            ratio,
            first,
            second,
        }))
    }

    /// All cell ids in pre-order.
    pub fn cell_ids(&self) -> Vec<CellId> {
        let mut ids = Vec::new();
        self.collect_cell_ids(&mut ids);
        ids
    }

    fn collect_cell_ids(&self, ids: &mut Vec<CellId>) {
        match self {
            LayoutNode::Cell(id) => ids.push(*id),
            LayoutNode::Split(split) => {
                split.first.collect_cell_ids(ids);
                split.second.collect_cell_ids(ids);
            }
        }
    }

    /// Number of cells (leaves).
    pub fn cell_count(&self) -> usize {
        match self {
            LayoutNode::Cell(_) => 1,
            LayoutNode::Split(split) => split.first.cell_count() + split.second.cell_count(),
        }
    }

    /// Renumbers all cells `0..n` in pre-order.
    pub(crate) fn renumber(&mut self) {
        fn walk(node: &mut LayoutNode, next: &mut u32) {
            match node {
                LayoutNode::Cell(id) => {
                    *id = CellId(*next);
                    *next += 1;
                }
                LayoutNode::Split(split) => {
                    walk(&mut split.first, next);
                    walk(&mut split.second, next);
                }
            }
        }
        walk(self, &mut 0);
    }

    /// Number of gaps along `dir` on the longest path through this subtree.
    fn gaps(&self, dir: SplitDir) -> u32 {
        match self {
            LayoutNode::Cell(_) => 0,
            LayoutNode::Split(split) if split.dir == dir => {
                1 + split.first.gaps(dir) + split.second.gaps(dir)
            }
            LayoutNode::Split(split) => split.first.gaps(dir).max(split.second.gaps(dir)),
        }
    }

    /// Smallest extent along `dir` at which every cell in this subtree is at least [`MIN_CELL_SIZE`] long,
    /// keeping the inner ratios fixed. Infinite if an inner ratio makes that impossible.
    fn required_extent(&self, dir: SplitDir, spacing: f32) -> f32 {
        match self {
            LayoutNode::Cell(_) => MIN_CELL_SIZE,
            LayoutNode::Split(split) if split.dir == dir => {
                let first_gaps = split.first.gaps(dir) as f32 * spacing;
                let second_gaps = split.second.gaps(dir) as f32 * spacing;
                let all_gaps = spacing + first_gaps + second_gaps;
                let needed_content = |required: f32, gaps: f32, share: f32| {
                    if share <= f32::EPSILON {
                        f32::INFINITY
                    } else {
                        ((required - gaps) / share).max(0.0)
                    }
                };
                let content = needed_content(
                    split.first.required_extent(dir, spacing),
                    first_gaps,
                    split.ratio,
                )
                .max(needed_content(
                    split.second.required_extent(dir, spacing),
                    second_gaps,
                    1.0 - split.ratio,
                ));
                content + all_gaps
            }
            LayoutNode::Split(split) => split
                .first
                .required_extent(dir, spacing)
                .max(split.second.required_extent(dir, spacing)),
        }
    }

    /// Rectangles of all cells in pre-order.
    pub fn cell_rects(&self, bounds: Rect, spacing: f32) -> Vec<(CellId, Rect)> {
        let mut out = Vec::new();
        self.collect_cell_rects(bounds, spacing, &mut out);
        out
    }

    fn collect_cell_rects(&self, bounds: Rect, spacing: f32, out: &mut Vec<(CellId, Rect)>) {
        match self {
            LayoutNode::Cell(id) => out.push((*id, bounds)),
            LayoutNode::Split(split) => {
                let (first, _, second) = split.rects(bounds, spacing);
                split.first.collect_cell_rects(first, spacing, out);
                split.second.collect_cell_rects(second, spacing, out);
            }
        }
    }

    /// All dividers in pre-order of their splits.
    pub fn dividers(&self, bounds: Rect, spacing: f32) -> Vec<Divider> {
        let mut out = Vec::new();
        self.collect_dividers(bounds, spacing, &mut out);
        out
    }

    fn collect_dividers(&self, bounds: Rect, spacing: f32, out: &mut Vec<Divider>) {
        if let LayoutNode::Split(split) = self {
            let (first, gap, second) = split.rects(bounds, spacing);
            out.push(Divider {
                id: DividerId(out.len() as u32),
                dir: split.dir,
                rect: gap,
            });
            split.first.collect_dividers(first, spacing, out);
            split.second.collect_dividers(second, spacing, out);
        }
    }

    /// Moves a divider by `delta` (collage units; only the component along the split axis is used).
    /// The movement is clamped so that no cell becomes smaller than [`MIN_CELL_SIZE`] along that axis.
    /// If the available space is too small to satisfy that, the divider stays where it is.
    pub fn drag_divider(
        &mut self,
        bounds: Rect,
        spacing: f32,
        id: DividerId,
        delta: Vec2,
    ) -> Result<(), CoreError> {
        let (split, split_bounds) = self
            .find_split_mut(bounds, spacing, id.0, &mut 0)
            .ok_or(CoreError::UnknownDivider(id))?;
        let content = split.content(&split_bounds, spacing);
        if content <= f32::EPSILON {
            return Ok(());
        }
        let first_gaps = split.first.gaps(split.dir) as f32 * spacing;
        let second_gaps = split.second.gaps(split.dir) as f32 * spacing;
        let min_ratio = (split.first.required_extent(split.dir, spacing) - first_gaps) / content;
        let max_ratio =
            1.0 - (split.second.required_extent(split.dir, spacing) - second_gaps) / content;
        let (lo, hi) = (min_ratio.max(0.0), max_ratio.min(1.0));
        if lo.is_nan() || hi.is_nan() || lo > hi {
            return Ok(());
        }
        let target = split.ratio + split.dir.component(delta) / content;
        // ±inf is clamped to the edge like any large value; NaN is ignored.
        if !target.is_nan() {
            split.ratio = target.clamp(lo, hi);
        }
        Ok(())
    }

    fn find_split_mut(
        &mut self,
        bounds: Rect,
        spacing: f32,
        target: u32,
        counter: &mut u32,
    ) -> Option<(&mut Split, Rect)> {
        let LayoutNode::Split(split) = self else {
            return None;
        };
        if *counter == target {
            return Some((split, bounds));
        }
        *counter += 1;
        let (first, _, second) = split.rects(bounds, spacing);
        if let Some(found) = split.first.find_split_mut(first, spacing, target, counter) {
            return Some(found);
        }
        split
            .second
            .find_split_mut(second, spacing, target, counter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn three_columns() -> LayoutNode {
        let mut node = LayoutNode::split(
            SplitDir::LeftRight,
            1.0 / 3.0,
            LayoutNode::cell(0),
            LayoutNode::split(
                SplitDir::LeftRight,
                0.5,
                LayoutNode::cell(0),
                LayoutNode::cell(0),
            ),
        );
        node.renumber();
        node
    }

    #[test]
    fn single_cell_fills_bounds() {
        let bounds = Rect::new(0.1, 0.2, 0.8, 0.6);
        let rects = LayoutNode::cell(0).cell_rects(bounds, 0.05);
        assert_eq!(rects, vec![(CellId(0), bounds)]);
    }

    #[test]
    fn chained_splits_give_equal_cells_with_spacing() {
        let spacing = 0.05;
        let rects = three_columns().cell_rects(Rect::new(0.0, 0.0, 1.0, 0.5), spacing);
        assert_eq!(rects.len(), 3);
        let expected = (1.0 - 2.0 * spacing) / 3.0;
        for (_, r) in &rects {
            assert!(
                close(r.width, expected),
                "width {} != {}",
                r.width,
                expected
            );
            assert!(close(r.height, 0.5));
        }
        // Gaps between neighbours are exactly `spacing`, last cell ends at the right edge.
        assert!(close(rects[1].1.x - rects[0].1.right(), spacing));
        assert!(close(rects[2].1.x - rects[1].1.right(), spacing));
        assert!(close(rects[2].1.right(), 1.0));
    }

    #[test]
    fn cells_do_not_overlap() {
        let node = LayoutNode::split(
            SplitDir::TopBottom,
            0.4,
            three_columns(),
            LayoutNode::cell(3),
        );
        let rects = node.cell_rects(Rect::new(0.0, 0.0, 1.0, 1.0), 0.02);
        for (i, (_, a)) in rects.iter().enumerate() {
            for (_, b) in rects.iter().skip(i + 1) {
                let overlap_x = a.x < b.right() - EPS && b.x < a.right() - EPS;
                let overlap_y = a.y < b.bottom() - EPS && b.y < a.bottom() - EPS;
                assert!(!(overlap_x && overlap_y), "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn renumber_assigns_ids_in_pre_order() {
        let ids = three_columns().cell_ids();
        assert_eq!(ids, vec![CellId(0), CellId(1), CellId(2)]);
    }

    #[test]
    fn dividers_are_listed_in_pre_order() {
        let dividers = three_columns().dividers(Rect::new(0.0, 0.0, 1.0, 1.0), 0.03);
        assert_eq!(dividers.len(), 2);
        assert_eq!(dividers[0].id, DividerId(0));
        assert!(dividers[0].rect.x < dividers[1].rect.x);
        assert!(close(dividers[0].rect.width, 0.03));
        assert!(close(dividers[0].rect.height, 1.0));
    }

    #[test]
    fn dragging_divider_moves_boundary() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut node = LayoutNode::split(
            SplitDir::LeftRight,
            0.5,
            LayoutNode::cell(0),
            LayoutNode::cell(1),
        );
        node.drag_divider(bounds, 0.0, DividerId(0), Vec2::new(0.1, 0.3))
            .unwrap();
        let rects = node.cell_rects(bounds, 0.0);
        assert!(close(rects[0].1.width, 0.6));
        assert!(close(rects[1].1.width, 0.4));
    }

    #[test]
    fn dragging_divider_respects_min_cell_size() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let spacing = 0.02;
        let mut node = three_columns();
        node.drag_divider(bounds, spacing, DividerId(0), Vec2::new(10.0, 0.0))
            .unwrap();
        for (_, r) in node.cell_rects(bounds, spacing) {
            assert!(r.width >= MIN_CELL_SIZE - EPS, "cell too narrow: {r:?}");
        }
        node.drag_divider(bounds, spacing, DividerId(0), Vec2::new(-10.0, 0.0))
            .unwrap();
        for (_, r) in node.cell_rects(bounds, spacing) {
            assert!(r.width >= MIN_CELL_SIZE - EPS, "cell too narrow: {r:?}");
        }
    }

    #[test]
    fn dragging_nested_divider_only_changes_its_split() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut node = three_columns();
        let before = node.cell_rects(bounds, 0.0);
        node.drag_divider(bounds, 0.0, DividerId(1), Vec2::new(0.1, 0.0))
            .unwrap();
        let after = node.cell_rects(bounds, 0.0);
        assert_eq!(before[0], after[0]);
        assert!(close(after[1].1.width, before[1].1.width + 0.1));
    }

    #[test]
    fn dragging_unknown_divider_fails() {
        let mut node = LayoutNode::cell(0);
        let result = node.drag_divider(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            0.0,
            DividerId(0),
            Vec2::default(),
        );
        assert_eq!(result, Err(CoreError::UnknownDivider(DividerId(0))));
    }

    #[test]
    fn dragging_with_no_room_keeps_ratio() {
        // Far too small for two cells of MIN_CELL_SIZE.
        let bounds = Rect::new(0.0, 0.0, 0.06, 1.0);
        let mut node = LayoutNode::split(
            SplitDir::LeftRight,
            0.5,
            LayoutNode::cell(0),
            LayoutNode::cell(1),
        );
        node.drag_divider(bounds, 0.0, DividerId(0), Vec2::new(0.01, 0.0))
            .unwrap();
        let LayoutNode::Split(split) = &node else {
            panic!("expected split")
        };
        assert!(close(split.ratio(), 0.5));
    }

    #[test]
    fn split_clamps_invalid_ratio() {
        let node = LayoutNode::split(
            SplitDir::TopBottom,
            f32::NAN,
            LayoutNode::cell(0),
            LayoutNode::cell(1),
        );
        let LayoutNode::Split(split) = &node else {
            panic!("expected split")
        };
        assert!(close(split.ratio(), 0.5));
    }

    #[test]
    fn dragging_top_bottom_divider_with_spacing_is_exact() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let spacing = 0.04;
        let mut node = LayoutNode::split(
            SplitDir::TopBottom,
            0.5,
            LayoutNode::cell(0),
            LayoutNode::cell(1),
        );
        node.drag_divider(bounds, spacing, DividerId(0), Vec2::new(0.5, -0.1))
            .unwrap();
        let rects = node.cell_rects(bounds, spacing);
        assert!(close(rects[0].1.height, 0.48 - 0.1));
        assert!(close(rects[1].1.height, 0.48 + 0.1));
        assert!(close(rects[0].1.width, 1.0));
        assert!(close(rects[1].1.y - rects[0].1.bottom(), spacing));
    }

    #[test]
    fn dragging_clamps_exactly_at_min_cell_size() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let spacing = 0.02;
        let mut node = three_columns();
        node.drag_divider(bounds, spacing, DividerId(0), Vec2::new(10.0, 0.0))
            .unwrap();
        let rects = node.cell_rects(bounds, spacing);
        // The two cells right of the divider share the minimum: the smaller one is exactly MIN_CELL_SIZE.
        let smallest = rects[1].1.width.min(rects[2].1.width);
        assert!(close(smallest, MIN_CELL_SIZE), "{smallest}");
        node.drag_divider(bounds, spacing, DividerId(0), Vec2::new(-10.0, 0.0))
            .unwrap();
        assert!(close(
            node.cell_rects(bounds, spacing)[0].1.width,
            MIN_CELL_SIZE
        ));
    }

    #[test]
    fn infinite_delta_clamps_and_nan_is_ignored() {
        let bounds = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut node = LayoutNode::split(
            SplitDir::LeftRight,
            0.5,
            LayoutNode::cell(0),
            LayoutNode::cell(1),
        );
        node.drag_divider(bounds, 0.0, DividerId(0), Vec2::new(f32::NAN, 0.0))
            .unwrap();
        assert!(close(node.cell_rects(bounds, 0.0)[0].1.width, 0.5));
        node.drag_divider(bounds, 0.0, DividerId(0), Vec2::new(f32::INFINITY, 0.0))
            .unwrap();
        assert!(close(
            node.cell_rects(bounds, 0.0)[1].1.width,
            MIN_CELL_SIZE
        ));
    }
}
