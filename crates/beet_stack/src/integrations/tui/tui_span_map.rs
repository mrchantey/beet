//! Maps terminal cell positions to entities for input hit-testing.
//!
//! [`TuiSpanMap`] is populated during rendering by [`TuiRenderer`](super::TuiRenderer)
//! and consumed by the input system to determine which entity the
//! mouse is interacting with.
use beet_core::prelude::*;

/// Maps terminal `(col, row)` positions to the entity that rendered content there.
///
/// Cleared and rebuilt each frame by the draw system. The input
/// system reads it to resolve mouse positions into entity targets
/// for [`TuiMouseDown`](super::TuiMouseDown) and related events.
#[derive(Debug, Default, Clone, Resource)]
pub struct TuiSpanMap {
	entries: HashMap<(u16, u16), Entity>,
}

impl TuiSpanMap {
	/// Remove all position mappings, typically at the start of each frame.
	pub fn clear(&mut self) { self.entries.clear(); }

	/// Assign every cell in `area` to `entity`, overwriting previous mappings.
	pub fn set_area(&mut self, area: ratatui::prelude::Rect, entity: Entity) {
		for row in area.y..area.y.saturating_add(area.height) {
			for col in area.x..area.x.saturating_add(area.width) {
				self.entries.insert((col, row), entity);
			}
		}
	}

	/// Look up the entity that owns the cell at `(col, row)`.
	pub fn get(&self, col: u16, row: u16) -> Option<Entity> {
		self.entries.get(&(col, row)).copied()
	}

	/// The number of mapped cells, useful for testing.
	pub fn len(&self) -> usize { self.entries.len() }

	/// Whether the map contains any entries.
	pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

#[cfg(test)]
mod test {
	use super::*;
	use ratatui::prelude::Rect;

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
				map.get(col, row).xpect_eq(Some(entity));
			}
		}
		// Outside the area should return None
		map.get(0, 0).xpect_eq(None);
		map.get(6, 3).xpect_eq(None);
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

		map.get(2, 0).xpect_eq(Some(first));
		map.get(3, 0).xpect_eq(Some(second));
		map.get(6, 0).xpect_eq(Some(second));
		map.get(7, 0).xpect_eq(Some(first));
	}
}
