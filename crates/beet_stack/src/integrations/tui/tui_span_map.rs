//! Maps terminal cell positions to entities for input hit-testing.
//!
//! [`TuiArea`] wraps a [`TuiSpanMap`] together with layout metadata
//! (outer rect, inner rect, scroll offset) so the input system can
//! translate terminal-space mouse coordinates into content-space
//! positions before looking up entities.
//!
//! [`TuiSpanMap`] is populated during rendering by [`TuiRenderer`](super::TuiRenderer)
//! and consumed by the input system to determine which entity the
//! mouse is interacting with.
use beet_core::prelude::*;
use ratatui::prelude::Rect;

/// A terminal cell position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TuiPos {
	pub row: u16,
	pub col: u16,
}

impl TuiPos {
	pub fn new(row: u16, col: u16) -> Self { Self { row, col } }
}


/// Maps terminal cell positions to the entity that rendered content there.
///
/// Cleared and rebuilt each frame by the draw system. Coordinates stored
/// here are in **content-buffer space** (0-based), not terminal space.
/// Use [`TuiArea::get`] to translate from terminal coordinates.
#[derive(Debug, Default, Clone)]
pub struct TuiSpanMap {
	entries: HashMap<TuiPos, Entity>,
}

impl TuiSpanMap {
	/// Remove all position mappings, typically at the start of each frame.
	pub fn clear(&mut self) { self.entries.clear(); }

	/// Assign every cell in `area` to `entity`, overwriting previous mappings.
	pub fn set_area(&mut self, area: Rect, entity: Entity) {
		for row in area.y..area.y.saturating_add(area.height) {
			for col in area.x..area.x.saturating_add(area.width) {
				self.entries.insert(TuiPos::new(row, col), entity);
			}
		}
	}

	/// Look up the entity that owns the cell at the given
	/// **content-buffer-space** position.
	pub fn get(&self, pos: TuiPos) -> Option<Entity> {
		self.entries.get(&pos).copied()
	}

	/// The number of mapped cells, useful for testing.
	pub fn len(&self) -> usize { self.entries.len() }

	/// Whether the map contains any entries.
	pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}


/// Wraps a [`TuiSpanMap`] with the layout context needed to translate
/// terminal-space mouse coordinates into content-space positions.
///
/// The renderer populates the span map in content-buffer space (0-based),
/// but mouse events arrive in terminal space. `TuiArea` stores the outer
/// rect (full terminal area including border), the inner rect (content
/// viewport inside the border), and the scroll offset so that [`Self::get`]
/// can perform the translation.
#[derive(Debug, Default, Clone, Resource)]
pub struct TuiArea {
	/// The span map populated by the renderer in content-buffer space.
	span_map: TuiSpanMap,
	/// The full terminal rect including border chrome.
	outer_rect: Rect,
	/// The inner content viewport rect (inside the border).
	inner_rect: Rect,
	/// Vertical scroll offset in content rows.
	scroll_offset: u16,
}

impl TuiArea {
	/// Create a new `TuiArea` with no coordinate translation.
	///
	/// Equivalent to an identity mapping where terminal space equals
	/// content space, ie no border and no scrolling.
	pub fn from_span_map(span_map: TuiSpanMap) -> Self {
		Self {
			span_map,
			outer_rect: Rect::default(),
			inner_rect: Rect::default(),
			scroll_offset: 0,
		}
	}

	/// Create a `TuiArea` with full layout context for coordinate
	/// translation.
	pub fn new(
		span_map: TuiSpanMap,
		outer_rect: Rect,
		inner_rect: Rect,
		scroll_offset: u16,
	) -> Self {
		Self {
			span_map,
			outer_rect,
			inner_rect,
			scroll_offset,
		}
	}

	/// Translate a terminal-space position to content-buffer space,
	/// then look up the entity.
	///
	/// Returns [`None`] if the position is outside the inner rect
	/// or if no entity is mapped at the translated position.
	pub fn get(&self, terminal_pos: TuiPos) -> Option<Entity> {
		// If we have no inner rect set (default), fall back to direct lookup
		if self.inner_rect.width == 0 && self.inner_rect.height == 0 {
			return self.span_map.get(terminal_pos);
		}

		let row = terminal_pos.row;
		let col = terminal_pos.col;

		// Check if the position is inside the inner (content viewport) rect
		if col < self.inner_rect.x
			|| col >= self.inner_rect.x.saturating_add(self.inner_rect.width)
			|| row < self.inner_rect.y
			|| row >= self.inner_rect.y.saturating_add(self.inner_rect.height)
		{
			return None;
		}

		// Translate terminal-space to content-buffer space:
		// - subtract inner_rect origin to get viewport-relative coords
		// - add scroll_offset to get content-buffer row
		let content_col = col.saturating_sub(self.inner_rect.x);
		let content_row = row
			.saturating_sub(self.inner_rect.y)
			.saturating_add(self.scroll_offset);

		self.span_map.get(TuiPos::new(content_row, content_col))
	}

	/// Direct access to the underlying span map.
	pub fn span_map(&self) -> &TuiSpanMap { &self.span_map }

	/// Mutable access to the underlying span map.
	pub fn span_map_mut(&mut self) -> &mut TuiSpanMap { &mut self.span_map }

	/// The outer terminal rect (including border).
	pub fn outer_rect(&self) -> Rect { self.outer_rect }

	/// The inner content viewport rect (inside border).
	pub fn inner_rect(&self) -> Rect { self.inner_rect }

	/// The vertical scroll offset.
	pub fn scroll_offset(&self) -> u16 { self.scroll_offset }

	/// Remove all position mappings from the inner span map.
	pub fn clear(&mut self) { self.span_map.clear(); }

	/// Whether the span map contains any entries.
	pub fn is_empty(&self) -> bool { self.span_map.is_empty() }
}


#[cfg(test)]
mod test {
	use super::*;
	use Rect;

	// -- TuiSpanMap tests --

	#[test]
	fn set_area_maps_all_cells() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		let area = Rect::new(2, 3, 4, 2);
		map.set_area(area, entity);

		// All cells in the 4x2 area should map to the entity
		map.len().xpect_eq(8);
		for row in 3..5 {
			for col in 2..6 {
				map.get(TuiPos::new(row, col)).xpect_eq(Some(entity));
			}
		}
		// Outside the area should return None
		map.get(TuiPos::new(0, 0)).xpect_eq(None);
		map.get(TuiPos::new(3, 6)).xpect_eq(None);
	}

	#[test]
	fn clear_removes_all_entries() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 10, 10), entity);
		map.is_empty().xpect_false();
		map.clear();
		map.is_empty().xpect_true();
	}

	#[test]
	fn later_entity_overwrites_earlier() {
		let mut world = World::new();
		let first = world.spawn_empty().id();
		let second = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 10, 2), first);
		// Overlapping area with a different entity
		map.set_area(Rect::new(3, 0, 4, 1), second);

		map.get(TuiPos::new(0, 2)).xpect_eq(Some(first));
		map.get(TuiPos::new(0, 3)).xpect_eq(Some(second));
		map.get(TuiPos::new(0, 6)).xpect_eq(Some(second));
		map.get(TuiPos::new(0, 7)).xpect_eq(Some(first));
	}

	// -- TuiArea tests --

	#[test]
	fn tui_area_identity_mapping() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 5, 1), entity);

		// No border, no scroll — terminal coords equal content coords
		let area = TuiArea::from_span_map(map);
		area.get(TuiPos::new(0, 0)).xpect_eq(Some(entity));
		area.get(TuiPos::new(0, 4)).xpect_eq(Some(entity));
		area.get(TuiPos::new(0, 5)).xpect_eq(None);
	}

	#[test]
	fn tui_area_translates_border_offset() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		// Content rendered at content-buffer (0,0)
		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 3, 1), entity);

		// Terminal: outer = 10x10 at (0,0), inner = 8x8 at (1,1)
		let outer = Rect::new(0, 0, 10, 10);
		let inner = Rect::new(1, 1, 8, 8);
		let area = TuiArea::new(map, outer, inner, 0);

		// Terminal (1,1) maps to content (0,0) — the entity
		area.get(TuiPos::new(1, 1)).xpect_eq(Some(entity));
		area.get(TuiPos::new(1, 3)).xpect_eq(Some(entity));
		// Terminal (1,4) maps to content (0,3) — past the 3-wide span
		area.get(TuiPos::new(1, 4)).xpect_eq(None);
		// Terminal (0,0) is on the border — outside inner rect
		area.get(TuiPos::new(0, 0)).xpect_eq(None);
		// Terminal (9,9) is on the border — outside inner rect
		area.get(TuiPos::new(9, 9)).xpect_eq(None);
	}

	#[test]
	fn tui_area_translates_scroll_offset() {
		let mut world = World::new();
		let entity_top = world.spawn_empty().id();
		let entity_bottom = world.spawn_empty().id();

		// Content: entity_top at row 0, entity_bottom at row 10
		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 5, 1), entity_top);
		map.set_area(Rect::new(0, 10, 5, 1), entity_bottom);

		// Inner viewport is 5x5 at (1,1), scrolled by 8 rows
		let outer = Rect::new(0, 0, 7, 7);
		let inner = Rect::new(1, 1, 5, 5);
		let area = TuiArea::new(map, outer, inner, 8);

		// Terminal row 1 + scroll 8 = content row 8 — no entity there
		area.get(TuiPos::new(1, 1)).xpect_eq(None);
		// Terminal row 3 + scroll 8 = content row 10 — entity_bottom
		area.get(TuiPos::new(3, 1)).xpect_eq(Some(entity_bottom));
	}

	#[test]
	fn tui_area_outside_inner_returns_none() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 80, 24), entity);

		let outer = Rect::new(0, 0, 80, 24);
		let inner = Rect::new(1, 1, 78, 22);
		let area = TuiArea::new(map, outer, inner, 0);

		// Positions on the border should return None
		area.get(TuiPos::new(0, 0)).xpect_eq(None);
		area.get(TuiPos::new(0, 40)).xpect_eq(None);
		area.get(TuiPos::new(23, 0)).xpect_eq(None);
		// Position inside should resolve
		area.get(TuiPos::new(1, 1)).xpect_eq(Some(entity));
	}

	#[test]
	fn tui_area_clear_delegates() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		let mut map = TuiSpanMap::default();
		map.set_area(Rect::new(0, 0, 5, 5), entity);

		let mut area = TuiArea::from_span_map(map);
		area.is_empty().xpect_false();
		area.clear();
		area.is_empty().xpect_true();
	}
}
