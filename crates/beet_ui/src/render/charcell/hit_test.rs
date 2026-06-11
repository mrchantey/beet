//! Cursor hit-testing and pointer/scroll input.
//!
//! Resolves a cursor cell to the entity under it (topmost in stacking order,
//! through the same scroll transform and clip the paint applied), fires the
//! renderer-agnostic [`Pointer*`](crate::prelude::PointerDown) events and
//! maintains [`Pointer::hover`], and routes wheel/keyboard scrolling to the
//! hovered/active scroll container.
//!
//! The hit-test reuses the charcell stacking order and paint-context transform
//! (so clicks land exactly where paint drew), but it reads only agnostic
//! components ([`LayoutRect`], [`ScrollPosition`], position/overflow) and fires
//! agnostic events, so the native renderer reuses the same pattern over its own
//! rects.

use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::math::IVec2;
use bevy::math::UVec2;
use bevy::math::Vec2;
use bevy::window::CursorMoved;

/// Lines a wheel notch and an arrow key scroll (mirrors the old TUI constants).
const MOUSE_SCROLL_LINES: i32 = 3;
const KEY_SCROLL_LINES: i32 = 1;
const PAGE_SCROLL_LINES: i32 = 10;

/// Per-buffer hit-test substrate: the tree, node data, and viewport needed to map
/// a cursor cell to the topmost entity under it. A renderer-agnostic resolver
/// over [`LayoutRect`] and the scroll transform, reusable by another renderer.
#[derive(SystemParam)]
pub struct HitTest<'w, 's> {
	charcell: CharcellQuery<'w, 's>,
	tree: CharcellTree<'w, 's>,
	roots: Query<'w, 's, (Entity, &'static DoubleBuffer)>,
}

impl HitTest<'_, '_> {
	/// The topmost entity at `cell`, resolved through every buffer root.
	///
	/// Prefers the entity that actually painted the cell (recorded on the painted
	/// [`Cell`]), which precisely accounts for inline flow (so a click on inline
	/// `<a>` text lands on the link), stacking, clipping, and the scroll transform.
	/// Falls back to a geometric rect walk for cells no glyph painted (eg a
	/// transparent container's padding).
	fn entity_at(&self, cell: IVec2) -> Option<Entity> {
		// the precisely-painted entity at this cell, if any glyph painted it.
		if cell.x >= 0 && cell.y >= 0 {
			let pos = UVec2::new(cell.x as u32, cell.y as u32);
			for (_, buffer) in self.roots.iter() {
				if let Some(entity) =
					buffer.front_buffer().get(pos).and_then(|cell| cell.entity)
				{
					return Some(entity);
				}
			}
		}
		for (root, buffer) in self.roots.iter() {
			let viewport = buffer.current_buffer().size();
			let ordered = self.tree.pre_order(root);
			let contexts = resolve_contexts(
				root,
				&ordered,
				&self.charcell,
				&self.tree,
				viewport,
			);
			let painted =
				stacking_order(root, &self.charcell, &self.tree, &managed_set(
					root,
					&self.charcell,
					&self.tree,
				));
			// topmost wins: scan the back-to-front order in reverse.
			if let Some(entity) = painted.iter().rev().find(|&&entity| {
				hit(entity, cell, &contexts, &self.charcell)
			}) {
				return Some(*entity);
			}
		}
		None
	}
}

/// Whether `entity`'s scroll-transformed, clipped rect contains `cell`.
fn hit(
	entity: Entity,
	cell: IVec2,
	contexts: &HashMap<Entity, PaintContext>,
	query: &CharcellQuery,
) -> bool {
	let Ok(node) = query.unresolved_node(entity) else {
		return false;
	};
	// display:none nodes have no painted rect; a zero-area rect can't be hit.
	let rect = node.layout_rect();
	if rect.is_empty() {
		return false;
	}
	let cx = contexts.get(&entity).copied().unwrap_or(PaintContext {
		clip: Clip::NONE,
		offset: IVec2::ZERO,
	});
	let screen = translate_rect(rect, cx.offset);
	// the clip keeps a scrolled-away cell unhittable, exactly as it is unpainted.
	cx.clip.cell(cell).is_some()
		&& cell.x >= screen.min.x
		&& cell.x < screen.max.x
		&& cell.y >= screen.min.y
		&& cell.y < screen.max.y
}

/// Recompute the inline-flow `managed` skip-set (descendants painted by their IFC
/// owner), so the hit-test stacking order matches paint.
fn managed_set(
	root: Entity,
	query: &CharcellQuery,
	tree: &CharcellTree,
) -> HashSet<Entity> {
	let mut managed = HashSet::<Entity>::default();
	for entity in tree.pre_order(root) {
		if managed.contains(&entity) {
			continue;
		}
		if let Ok(node) = query.unresolved_node(entity) {
			if establishes_inline_flow(&node, query) {
				managed.extend(tree.descendants(entity));
			}
		}
	}
	managed
}

/// ECS system: dispatch [`Pointer*`](crate::prelude::PointerDown) events from the
/// bridged bevy mouse input, maintaining [`Pointer::hover`].
///
/// `CursorMoved` updates hover (firing `PointerOver`/`PointerOut` on enter/leave),
/// `MouseButtonInput` fires `PointerDown`/`PointerUp` on the hit entity. The
/// events auto-propagate up the tree, so a click on text inside an `<a>` reaches
/// the `<a>` observer.
pub fn pointer_input(
	mut cursor: MessageReader<CursorMoved>,
	mut buttons: MessageReader<MouseButtonInput>,
	hit_test: HitTest,
	mut commands: Commands,
	mut pointers: Query<(Entity, &mut Pointer), With<PrimaryPointer>>,
	mut last_cursor: Local<Option<IVec2>>,
) -> Result {
	let Ok((pointer, mut hover)) = pointers.single_mut() else {
		cursor.clear();
		buttons.clear();
		return Ok(());
	};

	// hover follows the latest cursor position this frame.
	for moved in cursor.read() {
		let cell = vec2_to_cell(moved.position);
		*last_cursor = Some(cell);
		let target = hit_test.entity_at(cell);
		match (hover.hover, target) {
			(Some(old), Some(new)) if old != new => {
				commands.entity(old).trigger(PointerOut::new(pointer));
				commands.entity(new).trigger(PointerOver::new(pointer));
				hover.hover = Some(new);
			}
			(None, Some(new)) => {
				commands.entity(new).trigger(PointerOver::new(pointer));
				hover.hover = Some(new);
			}
			(Some(old), None) => {
				commands.entity(old).trigger(PointerOut::new(pointer));
				hover.hover = None;
			}
			_ => {}
		}
	}

	// button presses target the entity under the most recent cursor cell.
	for button in buttons.read() {
		let Some(cell) = *last_cursor else { continue };
		let Some(target) = hit_test.entity_at(cell) else {
			continue;
		};
		match button.state {
			ButtonState::Pressed => {
				commands.entity(target).trigger(PointerDown::new(pointer));
			}
			ButtonState::Released => {
				commands.entity(target).trigger(PointerUp::new(pointer));
			}
		}
	}
	Ok(())
}

/// ECS system: scroll the hovered scroll container on wheel, the focused/active
/// one on keyboard (arrows/PageUp/PageDown/Home/End).
///
/// A wheel event scrolls the nearest scrollable ancestor of the hovered element
/// (DOM behavior); keys scroll the container under the pointer's hover. A
/// `ScrollPosition` change repaints via change detection.
pub fn scroll_input(
	mut wheel: MessageReader<MouseWheel>,
	mut keys: MessageReader<KeyboardInput>,
	pointers: Query<&Pointer, With<PrimaryPointer>>,
	parents: Query<&ChildOf>,
	// transclusion: a `RenderRef` holder is the charcell parent of the entity it
	// points at, so the ancestor walk can cross from transcluded content (eg a
	// page) up into the holder's container (eg the page-host scrollport).
	refs: Query<(Entity, &RenderRef)>,
	mut scrolls: Query<&mut ScrollPosition>,
) {
	// accumulate this frame's scroll delta in cells.
	let mut delta = IVec2::ZERO;
	for ev in wheel.read() {
		let lines = match ev.unit {
			MouseScrollUnit::Line => MOUSE_SCROLL_LINES,
			// pixel deltas are coarse here; treat each as one notch
			MouseScrollUnit::Pixel => MOUSE_SCROLL_LINES,
		};
		// wheel y is positive up (content moves down); scroll offset is opposite.
		delta.x -= ev.x.signum() as i32 * lines * (ev.x != 0.) as i32;
		delta.y -= ev.y.signum() as i32 * lines * (ev.y != 0.) as i32;
	}
	for key in keys.read().filter(|k| k.state == ButtonState::Pressed) {
		match key.key_code {
			KeyCode::ArrowDown => delta.y += KEY_SCROLL_LINES,
			KeyCode::ArrowUp => delta.y -= KEY_SCROLL_LINES,
			KeyCode::ArrowRight => delta.x += KEY_SCROLL_LINES,
			KeyCode::ArrowLeft => delta.x -= KEY_SCROLL_LINES,
			KeyCode::PageDown => delta.y += PAGE_SCROLL_LINES,
			KeyCode::PageUp => delta.y -= PAGE_SCROLL_LINES,
			_ => {}
		}
	}
	if delta == IVec2::ZERO {
		return;
	}

	// scroll the hovered element's nearest scrollable ancestor (inclusive). Walk
	// up ChildOf, stopping at the first entity that carries a ScrollPosition.
	let Some(hovered) = pointers.single().ok().and_then(|p| p.hover) else {
		return;
	};
	let mut current = Some(hovered);
	let container = loop {
		let Some(entity) = current else { break None };
		if scrolls.contains(entity) {
			break Some(entity);
		}
		// transclusion wins for *visual* ancestry: if a RenderRef holder renders
		// this entity in place, the holder (eg the page-host scrollport) is its
		// visual parent, even though its structural ChildOf points elsewhere (eg a
		// route entity under the router). Otherwise walk up ChildOf.
		current = refs
			.iter()
			.find(|(_, render_ref)| render_ref.0 == entity)
			.map(|(holder, _)| holder)
			.or_else(|| parents.get(entity).ok().map(|child_of| child_of.parent()));
	};
	if let Some(container) = container {
		if let Ok(mut scroll) = scrolls.get_mut(container) {
			// the clamp_scroll_positions system settles this into range next frame.
			let next = scroll.offset + delta;
			if next != scroll.offset {
				scroll.offset = next;
			}
		}
	}
}

/// Convert a bevy cursor [`Vec2`] (cell-space, 1:1) to a signed cell.
fn vec2_to_cell(position: Vec2) -> IVec2 {
	IVec2::new(position.x.floor() as i32, position.y.floor() as i32)
}

// ── Scrollbar mouse interaction ─────────────────────────────────────────────────

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
	/// The screen-space scrollbar geometry of every scroll container.
	fn geometries(&self) -> HashMap<Entity, ScrollbarGeometry> {
		let mut map = HashMap::<Entity, ScrollbarGeometry>::default();
		for (root, buffer) in self.roots.iter() {
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
				let offset =
					contexts.get(&entity).map(|cx| cx.offset).unwrap_or(IVec2::ZERO);
				if let Some(geometry) =
					scrollbar_geometry(&node, &self.charcell, viewport, offset)
				{
					map.insert(entity, geometry);
				}
			}
		}
		map
	}

	/// The bar geometry of `container` on `axis`, for live drag mapping.
	fn bar(&self, container: Entity, axis: ScrollAxis) -> Option<AxisBar> {
		self.geometries().get(&container).and_then(|geometry| match axis {
			ScrollAxis::Y => geometry.vertical,
			ScrollAxis::X => geometry.horizontal,
		})
	}

	/// The scrollbar press at `cell`, if any.
	fn hit(&self, cell: IVec2) -> Option<ScrollbarHit> {
		for (container, geometry) in self.geometries() {
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
fn bar_region(bar: &AxisBar, cross: i32, along: i32) -> Option<ScrollbarRegion> {
	if cross != bar.line {
		return None;
	}
	if along < bar.track_start || along >= bar.track_start + bar.track_len as i32 {
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
/// Sits alongside [`scroll_input`] (wheel/keys). A press in a gutter pages or
/// begins a thumb drag; a `CursorMoved` while dragging maps the cursor to a
/// clamped offset; a release ends the drag. A press outside every gutter is
/// ignored here and falls through to the normal pointer hit-test.
pub fn scrollbar_mouse(
	mut buttons: MessageReader<MouseButtonInput>,
	mut cursor: MessageReader<CursorMoved>,
	// the hit-test reads ScrollPosition (via CharcellQuery), so it cannot coexist
	// with the mutable write query; a ParamSet keeps the accesses disjoint.
	mut params: ParamSet<(ScrollbarHitTest, Query<&'static mut ScrollPosition>)>,
	mut drag: Local<Option<ScrollbarDrag>>,
	mut last_cursor: Local<Option<IVec2>>,
) {
	// phase 1 (read-only hit-test): collect the offset writes this frame implies.
	let mut writes = Vec::<ScrollWrite>::new();
	{
		let hit_test = params.p0();
		// a move while dragging maps the cursor along the track to a scroll offset.
		for moved in cursor.read() {
			let cell = vec2_to_cell(moved.position);
			*last_cursor = Some(cell);
			let Some(state) = drag.as_ref() else { continue };
			let Some(bar) = hit_test.bar(state.container, state.axis) else {
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
			match button.state {
				ButtonState::Pressed => {
					let Some(cell) = *last_cursor else { continue };
					let Some(hit) = hit_test.hit(cell) else { continue };
					match hit.region {
						ScrollbarRegion::Thumb { grab } => {
							*drag = Some(ScrollbarDrag {
								container: hit.container,
								axis: hit.axis,
								grab,
							});
						}
						// click the track to page one scrollport toward the click.
						ScrollbarRegion::PageBackward => {
							writes.push(ScrollWrite::page(&hit, -(hit.bar.track_len as i32)));
						}
						ScrollbarRegion::PageForward => {
							writes.push(ScrollWrite::page(&hit, hit.bar.track_len as i32));
						}
					}
				}
				// any release ends a drag (clamp_scroll_positions settles it).
				ButtonState::Released => *drag = None,
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
	Set { container: Entity, axis: ScrollAxis, offset: i32 },
	/// Add `delta` to an axis, clamped to `[0, max]` (a track page).
	Page { container: Entity, axis: ScrollAxis, delta: i32, max: i32 },
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
			Self::Set { container, .. } | Self::Page { container, .. } => *container,
		}
	}

	/// The new offset this write produces from the `current` offset.
	fn resolve(&self, current: IVec2) -> IVec2 {
		let mut next = current;
		match *self {
			Self::Set { axis, offset, .. } => set_axis_offset(&mut next, axis, offset),
			Self::Page { axis, delta, max, .. } => {
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

	/// Records the entities that received each pointer event, by an observer the
	/// tests attach, to assert which entity a click/hover resolved to.
	#[derive(Resource, Default)]
	struct PointerLog {
		down: Vec<Entity>,
		over: Vec<Entity>,
		out: Vec<Entity>,
	}

	/// An SGR mouse sequence: button `b` at 1-indexed cell `(col+1, row+1)`,
	/// pressed (`M`) or released (`m`).
	fn sgr(b: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
		let m = if pressed { 'M' } else { 'm' };
		format!("\x1b[<{b};{};{}{m}", col + 1, row + 1).into_bytes()
	}

	/// Boot a host with the pointer-logging observers attached.
	fn logging_host() -> TestHost {
		let mut host = TestHost::new();
		host.app.init_resource::<PointerLog>();
		host.app.add_observer(
			|ev: On<PointerDown>, mut log: ResMut<PointerLog>| {
				log.down.push(ev.event_target());
			},
		);
		host.app.add_observer(
			|ev: On<PointerOver>, mut log: ResMut<PointerLog>| {
				log.over.push(ev.event_target());
			},
		);
		host.app.add_observer(
			|ev: On<PointerOut>, mut log: ResMut<PointerLog>| {
				log.out.push(ev.event_target());
			},
		);
		host
	}

	/// A click resolves to (and auto-propagates to) the element under the cell.
	#[beet_core::test]
	fn click_fires_pointer_down_on_target() {
		let mut host = logging_host();
		// a labelled box; observe PointerDown on the box entity
		let target = host
			.app
			.world_mut()
			.spawn(rsx! { <div>"hello"</div> })
			.id();
		host.app.world_mut().entity_mut(host.host).add_child(target);
		host.step();
		// click the first cell, where "hello" begins
		host.send_input(&sgr(0, 0, 0, true));
		host.step();
		let log = host.app.world().resource::<PointerLog>();
		// the click propagated up to the div (the text node's ancestor)
		log.down.contains(&target).xpect_true();
	}

	/// Moving the cursor across two stacked rows fires Over/Out with the right
	/// targets and tracks `Pointer::hover`.
	#[beet_core::test]
	fn hover_tracks_over_and_out() {
		let mut host = logging_host();
		let first = host.app.world_mut().spawn(rsx! { <div>"AAAA"</div> }).id();
		let second = host.app.world_mut().spawn(rsx! { <div>"BBBB"</div> }).id();
		host.app.world_mut().entity_mut(host.host).add_child(first);
		host.app.world_mut().entity_mut(host.host).add_child(second);
		host.step();
		// hover the first row, then the second (motion events, button 35)
		host.send_input(&sgr(35, 1, 0, true));
		host.step();
		host.send_input(&sgr(35, 1, 1, true));
		host.step();
		let log = host.app.world().resource::<PointerLog>();
		log.over.contains(&first).xpect_true();
		log.over.contains(&second).xpect_true();
		log.out.contains(&first).xpect_true();
		// hover now tracks the second row's entity (or its text child)
		let hover = host
			.app
			.world_mut()
			.query_filtered::<&Pointer, With<PrimaryPointer>>()
			.single(host.app.world())
			.unwrap()
			.hover;
		hover.xpect_some();
	}

	/// A wheel-down while hovering a scroll container increases its scroll offset.
	#[beet_core::test]
	fn wheel_scrolls_hovered_container() {
		let mut host = logging_host();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("scroller")
					.with_value(common_props::Height, Length::Rem(3.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			],
		);
		let content = host
			.app
			.world_mut()
			.spawn(rsx! {
				<div class="scroller"><pre>"a\nb\nc\nd\ne\nf\ng\nh"</pre></div>
			})
			.id();
		host.app.world_mut().entity_mut(host.host).add_child(content);
		host.step();
		// hover over the scroller, then wheel down
		host.send_input(&sgr(35, 1, 1, true));
		host.step();
		host.send_input(&sgr(65, 1, 1, true)); // wheel down
		host.step();
		let offset_y = host
			.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.map(|s| s.offset.y)
			.max()
			.unwrap_or(0);
		(offset_y > 0).xpect_true();
	}

	/// A click inside a scrolled container hits the scroll-translated entity, not
	/// the one at the unscrolled position (proves the shared transform): the same
	/// cell resolves to a different row before and after scrolling.
	#[beet_core::test]
	fn click_in_scrolled_container_uses_transform() {
		let mut host = logging_host();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("scroller")
					.with_value(common_props::Height, Length::Rem(3.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			],
		);
		let scroller = host
			.app
			.world_mut()
			.spawn(rsx! {
				<div class="scroller">
					<div>"R0"</div>
					<div>"R1"</div>
					<div>"R2"</div>
					<div>"R3"</div>
					<div>"R4"</div>
				</div>
			})
			.id();
		host.app.world_mut().entity_mut(host.host).add_child(scroller);
		host.step();
		// click the top-left cell unscrolled, recording the deepest hit row.
		host.send_input(&sgr(0, 0, 0, true));
		host.step();
		let unscrolled_hit =
			host.app.world().resource::<PointerLog>().down.first().copied();
		host.app.world_mut().resource_mut::<PointerLog>().down.clear();

		// scroll down by 2 so a different row sits at the top, then click the same
		// cell again.
		let scroll_entity = host
			.app
			.world_mut()
			.query_filtered::<Entity, With<ScrollPosition>>()
			.single(host.app.world())
			.unwrap();
		host.app
			.world_mut()
			.entity_mut(scroll_entity)
			.insert(ScrollPosition::new(IVec2::new(0, 2)));
		host.step();
		host.send_input(&sgr(0, 0, 0, true));
		host.step();
		let scrolled_hit =
			host.app.world().resource::<PointerLog>().down.first().copied();

		// the same cell resolved to a different entity after scrolling: the hit-test
		// applied the scroll transform, not the unscrolled rect.
		unscrolled_hit.xpect_some();
		scrolled_hit.xpect_some();
		(unscrolled_hit != scrolled_hit).xpect_true();
	}

	// ── Scrollbar mouse interaction (Task 06) ──

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
		let col = bar.iter().map(|(x, _, _)| *x).max().expect("a vertical bar");
		let mut track: Vec<u32> =
			bar.iter().filter(|(x, _, _)| *x == col).map(|(_, y, _)| *y).collect();
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
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("scroller")
					.with_value(common_props::Height, Length::Rem(6.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			],
		);
		let body: String =
			(0..30).map(|i| format!("r{i}")).collect::<Vec<_>>().join("\n");
		host.spawn_content(rsx! { <div class="scroller"><pre>{body}</pre></div> });
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
		host.frame_plain().xpect_contains("r29").xnot().xpect_contains("r0");
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
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("wide")
					.with_value(common_props::Width, Length::Rem(8.))
					.with_value(common_props::OverflowXProp, Overflow::Scroll),
			],
		);
		// content wider than the 40-cell buffer, so it overflows horizontally.
		let wide: String = ('a'..='z').chain('A'..='Z').cycle().take(60).collect();
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
