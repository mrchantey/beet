//! Scrollbar mouse interaction: hit-test a cursor cell to a scrollbar region and
//! drive the container's scroll from track clicks and thumb drags.
//!
//! Sits alongside the pointer/scroll input in [`hit_test`](super::hit_test): the
//! [`scrollbar_mouse`] system claims gutter presses (paging the track, dragging
//! the thumb), and any press outside every gutter falls through to the normal
//! pointer hit-test. The geometry it acts on is the shared
//! [`scrollbar_geometry`](super::scrollbar_geometry), so a click lands exactly on
//! the painted bar.

use super::*;
// pin `ScrollPosition` to beet_ui's own: bevy's same-named ui type arrives through
// `beet_core::prelude` under `bevy_default` and would otherwise clash. It is the only
// item used from `crate::prelude`, so the explicit import replaces the glob.
use crate::prelude::ScrollPosition;
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::math::IVec2;
use bevy::window::CursorMoved;

/// The two scroll axes a scrollbar drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollAxis {
	/// The horizontal bar (bottom gutter), scrolling `offset.x`.
	X,
	/// The vertical bar (right gutter), scrolling `offset.y`.
	Y,
}

/// Which part of a scrollbar a press landed on.
#[derive(Debug, Clone, Copy)]
enum ScrollbarRegion {
	/// On the thumb: begin a drag, grabbing `grab` cells into the thumb.
	Thumb { grab: i32 },
	/// On the track before the thumb: page toward the start.
	PageBackward,
	/// On the track after the thumb: page toward the end.
	PageForward,
}

/// A scrollbar press: the container, the axis it scrolls, the region hit, and the
/// live bar geometry (its `track_len` pages, its `offset_at` maps a drag).
struct ScrollbarHit {
	container: Entity,
	axis: ScrollAxis,
	region: ScrollbarRegion,
	bar: AxisBar,
}

/// An in-progress thumb drag, persisted across frames (press → move → release)
/// as the [`scrollbar_mouse`] system's `Local` state.
pub struct ScrollbarDrag {
	container: Entity,
	axis: ScrollAxis,
	/// Cells the cursor sat into the thumb when grabbed, kept under the cursor.
	grab: i32,
}

/// Per-buffer scrollbar hit-test substrate: maps a cursor cell to the scrollbar
/// region under it, reusing the shared [`scrollbar_geometry`] so a click lands
/// exactly on the painted bar.
#[derive(SystemParam)]
pub struct ScrollbarHitTest<'w, 's> {
	charcell: CharcellQuery<'w, 's>,
	tree: CharcellTree<'w, 's>,
	roots: Query<'w, 's, (Entity, &'static DoubleBuffer)>,
}

impl ScrollbarHitTest<'_, '_> {
	/// The screen-space scrollbar geometry of every scroll container within
	/// `surface`'s buffer, so a press on one surface never drives another's bar.
	fn geometries_for(
		&self,
		surface: Entity,
	) -> HashMap<Entity, ScrollbarGeometry> {
		let mut map = HashMap::<Entity, ScrollbarGeometry>::default();
		let Ok((root, buffer)) = self.roots.get(surface) else {
			return map;
		};
		let viewport = buffer.current_buffer().size();
		let ordered = self.tree.pre_order(root);
		let contexts = resolve_contexts(
			root,
			&ordered,
			&self.charcell,
			&self.tree,
			viewport,
		);
		for entity in ordered {
			let Ok(node) = self.charcell.unresolved_node(entity) else {
				continue;
			};
			let offset = contexts
				.get(&entity)
				.map(|cx| cx.offset)
				.unwrap_or(IVec2::ZERO);
			if let Some(geometry) =
				scrollbar_geometry(&node, &self.charcell, viewport, offset)
			{
				map.insert(entity, geometry);
			}
		}
		map
	}

	/// The bar geometry of `container` on `axis` within `surface`, for live drag
	/// mapping.
	fn bar(
		&self,
		surface: Entity,
		container: Entity,
		axis: ScrollAxis,
	) -> Option<AxisBar> {
		self.geometries_for(surface)
			.get(&container)
			.and_then(|geometry| match axis {
				ScrollAxis::Y => geometry.vertical,
				ScrollAxis::X => geometry.horizontal,
			})
	}

	/// The scrollbar press at `cell` within `surface`, if any.
	fn hit_surface(
		&self,
		surface: Entity,
		cell: IVec2,
	) -> Option<ScrollbarHit> {
		for (container, geometry) in self.geometries_for(surface) {
			// vertical bar: cursor column on the gutter line, row along the track
			if let Some(bar) = geometry.vertical {
				if let Some(region) = bar_region(&bar, cell.x, cell.y) {
					return Some(ScrollbarHit {
						container,
						axis: ScrollAxis::Y,
						region,
						bar,
					});
				}
			}
			// horizontal bar: cursor row on the gutter line, column along the track
			if let Some(bar) = geometry.horizontal {
				if let Some(region) = bar_region(&bar, cell.y, cell.x) {
					return Some(ScrollbarHit {
						container,
						axis: ScrollAxis::X,
						region,
						bar,
					});
				}
			}
		}
		None
	}
}

/// Classify a press on `bar`: `cross` is the cursor's cross-axis coordinate
/// (compared to the gutter line), `along` the along-axis coordinate (compared to
/// the track and thumb spans). `None` when the cursor is off this bar.
fn bar_region(
	bar: &AxisBar,
	cross: i32,
	along: i32,
) -> Option<ScrollbarRegion> {
	if cross != bar.line {
		return None;
	}
	if along < bar.track_start
		|| along >= bar.track_start + bar.track_len as i32
	{
		return None;
	}
	if along < bar.thumb_start {
		Some(ScrollbarRegion::PageBackward)
	} else if along >= bar.thumb_start + bar.thumb_len as i32 {
		Some(ScrollbarRegion::PageForward)
	} else {
		Some(ScrollbarRegion::Thumb {
			grab: along - bar.thumb_start,
		})
	}
}

/// Read an axis from a scroll offset.
fn axis_offset(offset: IVec2, axis: ScrollAxis) -> i32 {
	match axis {
		ScrollAxis::X => offset.x,
		ScrollAxis::Y => offset.y,
	}
}

/// Write an axis of a scroll offset.
fn set_axis_offset(offset: &mut IVec2, axis: ScrollAxis, value: i32) {
	match axis {
		ScrollAxis::X => offset.x = value,
		ScrollAxis::Y => offset.y = value,
	}
}

/// ECS system: drive a container's scroll from mouse interaction with its
/// scrollbar, like a browser: click the track to page toward the click, drag the
/// thumb to scroll proportionally.
///
/// Sits alongside [`scroll_input`](super::scroll_input) (wheel/keys). A press in a
/// gutter pages or begins a thumb drag; a `CursorMoved` while dragging maps the
/// cursor to a clamped offset; a release ends the drag. A press outside every
/// gutter is ignored here and falls through to the normal pointer hit-test.
pub fn scrollbar_mouse(
	mut buttons: MessageReader<MouseButtonInput>,
	mut cursor: MessageReader<CursorMoved>,
	// the hit-test reads ScrollPosition (via CharcellQuery), so it cannot coexist
	// with the mutable write query; a ParamSet keeps the accesses disjoint.
	mut params: ParamSet<(
		ScrollbarHitTest,
		Query<&'static mut ScrollPosition>,
	)>,
	// drag + last cursor are kept per surface (window), so a drag on one SSH
	// session never moves another's scrollbar.
	mut drag: Local<HashMap<Entity, ScrollbarDrag>>,
	mut last_cursor: Local<HashMap<Entity, IVec2>>,
) {
	// phase 1 (read-only hit-test): collect the offset writes this frame implies.
	let mut writes = Vec::<ScrollWrite>::new();
	{
		let hit_test = params.p0();
		// a move while dragging maps the cursor along the track to a scroll offset.
		for moved in cursor.read() {
			let surface = moved.window;
			let cell = vec2_to_cell(moved.position);
			last_cursor.insert(surface, cell);
			let Some(state) = drag.get(&surface) else {
				continue;
			};
			let Some(bar) = hit_test.bar(surface, state.container, state.axis)
			else {
				continue;
			};
			let along = axis_offset(cell, state.axis);
			writes.push(ScrollWrite::Set {
				container: state.container,
				axis: state.axis,
				offset: bar.offset_at(along, state.grab),
			});
		}
		for button in buttons.read() {
			let surface = button.window;
			match button.state {
				ButtonState::Pressed => {
					let Some(&cell) = last_cursor.get(&surface) else {
						continue;
					};
					let Some(hit) = hit_test.hit_surface(surface, cell) else {
						continue;
					};
					match hit.region {
						ScrollbarRegion::Thumb { grab } => {
							drag.insert(surface, ScrollbarDrag {
								container: hit.container,
								axis: hit.axis,
								grab,
							});
						}
						// click the track to page one scrollport toward the click.
						ScrollbarRegion::PageBackward => {
							writes.push(ScrollWrite::page(
								&hit,
								-(hit.bar.track_len as i32),
							));
						}
						ScrollbarRegion::PageForward => {
							writes.push(ScrollWrite::page(
								&hit,
								hit.bar.track_len as i32,
							));
						}
					}
				}
				// any release ends this surface's drag (clamp settles it).
				ButtonState::Released => {
					drag.remove(&surface);
				}
			}
		}
	}

	// phase 2 (mutable): apply the collected offset writes.
	let mut scrolls = params.p1();
	for write in writes {
		let Ok(mut scroll) = scrolls.get_mut(write.container()) else {
			continue;
		};
		let next = write.resolve(scroll.offset);
		if next != scroll.offset {
			scroll.offset = next;
		}
	}
}

/// A pending scroll-offset mutation collected during the read-only hit-test, then
/// applied against the mutable [`ScrollPosition`] query.
enum ScrollWrite {
	/// Set an axis to an absolute offset (a thumb drag).
	Set {
		container: Entity,
		axis: ScrollAxis,
		offset: i32,
	},
	/// Add `delta` to an axis, clamped to `[0, max]` (a track page).
	Page {
		container: Entity,
		axis: ScrollAxis,
		delta: i32,
		max: i32,
	},
}

impl ScrollWrite {
	/// A page write from a scrollbar hit and a signed cell delta.
	fn page(hit: &ScrollbarHit, delta: i32) -> Self {
		Self::Page {
			container: hit.container,
			axis: hit.axis,
			delta,
			max: hit.bar.max_offset,
		}
	}

	fn container(&self) -> Entity {
		match self {
			Self::Set { container, .. } | Self::Page { container, .. } => {
				*container
			}
		}
	}

	/// The new offset this write produces from the `current` offset.
	fn resolve(&self, current: IVec2) -> IVec2 {
		let mut next = current;
		match *self {
			Self::Set { axis, offset, .. } => {
				set_axis_offset(&mut next, axis, offset)
			}
			Self::Page {
				axis, delta, max, ..
			} => {
				let value = (axis_offset(current, axis) + delta).clamp(0, max);
				set_axis_offset(&mut next, axis, value);
			}
		}
		next
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::render::charcell::test_host::TestHost;
	use crate::style::*;

	/// An SGR mouse sequence: button `b` at 1-indexed cell `(col+1, row+1)`,
	/// pressed (`M`) or released (`m`).
	fn sgr(b: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
		let m = if pressed { 'M' } else { 'm' };
		format!("\x1b[<{b};{};{}{m}", col + 1, row + 1).into_bytes()
	}

	/// The vertical scrollbar's painted column and its track + thumb rows, scanned
	/// from the rendered buffer so the test clicks exactly where paint drew.
	fn vbar(host: &TestHost) -> (u32, Vec<u32>, Vec<u32>) {
		let dbuf = host.app.world().get::<DoubleBuffer>(host.host).unwrap();
		let bar: Vec<(u32, u32, String)> = dbuf
			.front_buffer()
			.iter_cells()
			.filter(|(_, cell)| matches!(cell.symbol_str(), "│" | "█"))
			.map(|(pos, cell)| (pos.x, pos.y, cell.symbol_str().to_string()))
			.collect();
		let col = bar
			.iter()
			.map(|(x, _, _)| *x)
			.max()
			.expect("a vertical bar");
		let mut track: Vec<u32> = bar
			.iter()
			.filter(|(x, _, _)| *x == col)
			.map(|(_, y, _)| *y)
			.collect();
		let mut thumb: Vec<u32> = bar
			.iter()
			.filter(|(x, _, glyph)| *x == col && glyph == "█")
			.map(|(_, y, _)| *y)
			.collect();
		track.sort();
		thumb.sort();
		(col, track, thumb)
	}

	/// The maximum scroll offset across every container (y).
	fn offset_y(host: &mut TestHost) -> i32 {
		host.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.map(|scroll| scroll.offset.y)
			.max()
			.unwrap_or(0)
	}

	/// The maximum scroll offset across every container (x).
	fn offset_x(host: &mut TestHost) -> i32 {
		host.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.map(|scroll| scroll.offset.x)
			.max()
			.unwrap_or(0)
	}

	/// A tall vertical scroll container (30 rows in a 6-row scrollport).
	fn tall_scroller_host() -> TestHost {
		let mut host = TestHost::new();
		host.app
			.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(vec![
				Rule::class("scroller")
					.with_value(common_props::Height, Length::Rem(6.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			]);
		let body: String = (0..30)
			.map(|i| format!("r{i}"))
			.collect::<Vec<_>>()
			.join("\n");
		host.spawn_content(
			rsx! { <div class="scroller"><pre>{body}</pre></div> },
		);
		host.step();
		host
	}

	/// Clicking the track below the thumb pages the content forward by ~one page.
	#[beet_core::test]
	fn scrollbar_track_click_pages_forward() {
		let mut host = tall_scroller_host();
		offset_y(&mut host).xpect_eq(0);
		// at offset 0 the thumb sits at the top; click the bottom of the track.
		let (col, track, _thumb) = vbar(&host);
		let bottom = *track.last().unwrap();
		host.send_input(&sgr(0, col, bottom, true));
		host.step();
		// paged forward by ~one scrollport, revealing later content.
		(offset_y(&mut host) > 0).xpect_true();
		host.frame_plain().xnot().xpect_contains("r0");
	}

	/// Dragging the thumb to the bottom of the track scrolls to the maximum offset.
	#[beet_core::test]
	fn scrollbar_thumb_drag_scrolls_proportionally() {
		let mut host = tall_scroller_host();
		let (col, track, thumb) = vbar(&host);
		let (top, bottom) = (*track.first().unwrap(), *track.last().unwrap());
		// press the thumb (at the top), drag to the track bottom, release.
		host.send_input(&sgr(0, col, *thumb.first().unwrap(), true));
		host.step();
		host.send_input(&sgr(35, col, bottom, true));
		host.step();
		host.send_input(&sgr(0, col, bottom, false));
		host.step();
		// a full drag to the track bottom reaches the maximum offset: the last
		// content row is visible and the first is gone.
		let at_max = offset_y(&mut host);
		(at_max > 0).xpect_true();
		host.frame_plain()
			.xpect_contains("r29")
			.xnot()
			.xpect_contains("r0");
		// the release ended the drag: a bare move no longer scrolls.
		host.send_input(&sgr(35, col, top, true));
		host.step();
		offset_y(&mut host).xpect_eq(at_max);
	}

	/// The horizontal analog: clicking the bottom-gutter track right of the thumb
	/// pages the wide content rightward.
	#[beet_core::test]
	fn scrollbar_horizontal_track_click_pages() {
		let mut host = TestHost::new();
		host.app
			.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(vec![
				Rule::class("wide")
					.with_value(common_props::Width, Length::Rem(8.))
					.with_value(common_props::OverflowXProp, Overflow::Scroll),
			]);
		// content wider than the 40-cell buffer, so it overflows horizontally.
		let wide: String =
			('a'..='z').chain('A'..='Z').cycle().take(60).collect();
		host.spawn_content(rsx! {
			<div class="wide"><pre>{wide}</pre></div>
		});
		host.step();
		// the horizontal bar is the bottom row of box-drawing glyphs.
		let dbuf = host.app.world().get::<DoubleBuffer>(host.host).unwrap();
		let bar: Vec<(u32, u32)> = dbuf
			.front_buffer()
			.iter_cells()
			.filter(|(_, cell)| matches!(cell.symbol_str(), "─" | "█"))
			.map(|(pos, _)| (pos.x, pos.y))
			.collect();
		let row = bar.iter().map(|(_, y)| *y).max().expect("a horizontal bar");
		let right = bar
			.iter()
			.filter(|(_, y)| *y == row)
			.map(|(x, _)| *x)
			.max()
			.unwrap();
		// click the track right of the thumb -> page the content rightward.
		host.send_input(&sgr(0, right, row, true));
		host.step();
		(offset_x(&mut host) > 0).xpect_true();
	}
}
