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
	/// Walks the stacking order back-to-front and returns the last (topmost)
	/// entity whose scroll-transformed rect contains the cell and whose clip keeps
	/// the cell visible. Mirrors the paint, so a click lands on what was drawn.
	fn entity_at(&self, cell: IVec2) -> Option<Entity> {
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
		current = parents.get(entity).ok().map(|child_of| child_of.parent());
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
}
