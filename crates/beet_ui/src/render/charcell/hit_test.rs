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

/// ECS system: scroll on wheel and on the keyboard
/// (arrows/PageUp/PageDown/Home/End), like a browser.
///
/// The target container is resolved in priority order, mirroring the DOM:
/// 1. the nearest scrollable ancestor of the hovered element (a wheel sets its
///    own hover the same frame, so a wheel always lands here),
/// 2. else the nearest scrollable ancestor of the focused element (so the
///    keyboard scrolls the focused scrollable with nothing under the pointer),
/// 3. else the outermost scroll container of a buffer-root tree (the page
///    scrollport), so arrow/page keys scroll the document by default.
///
/// A `ScrollPosition` change repaints via change detection.
//
// crate-visible (not `pub`): it reads the crate-internal `CharcellTree`, like the
// `paint`/`prepare` systems. The plugin adds it via `super::*`.
pub(crate) fn scroll_input(
	mut wheel: MessageReader<MouseWheel>,
	mut keys: MessageReader<KeyboardInput>,
	pointers: Query<&Pointer, With<PrimaryPointer>>,
	focused: Query<Entity, With<Focus>>,
	parents: Query<&ChildOf>,
	// transclusion: a `RenderRef` holder is the charcell parent of the entity it
	// points at, so the ancestor walk can cross from transcluded content (eg a
	// page) up into the holder's container (eg the page-host scrollport).
	refs: Query<(Entity, &RenderRef)>,
	tree: CharcellTree,
	roots: Query<Entity, With<DoubleBuffer>>,
	mut scrolls: Query<&mut ScrollPosition>,
) {
	// accumulate this frame's scroll delta in cells; track whether any of it came
	// from the keyboard, which falls back to the focused/page scrollport.
	let mut delta = IVec2::ZERO;
	for ev in wheel.read() {
		let lines = match ev.unit {
			MouseScrollUnit::Line => MOUSE_SCROLL_LINES,
			// pixel deltas are coarse here; treat each as one notch
			MouseScrollUnit::Pixel => MOUSE_SCROLL_LINES,
		};
		// wheel y is positive up (content scrolls up), so the offset moves opposite;
		// wheel x is positive right (content scrolls right), so the offset follows it.
		delta.x += ev.x.signum() as i32 * lines * (ev.x != 0.) as i32;
		delta.y -= ev.y.signum() as i32 * lines * (ev.y != 0.) as i32;
	}
	let pressed = keys
		.read()
		.filter(|key| key.state == ButtonState::Pressed)
		.map(|key| key.key_code)
		.collect::<Vec<_>>();
	// alt+arrows are reserved for history nav (back/forward), not scrolling.
	let alt = pressed
		.iter()
		.any(|key| matches!(key, KeyCode::AltLeft | KeyCode::AltRight));
	for key in &pressed {
		match key {
			KeyCode::ArrowLeft | KeyCode::ArrowRight if alt => {}
			KeyCode::ArrowDown => delta.y += KEY_SCROLL_LINES,
			KeyCode::ArrowUp => delta.y -= KEY_SCROLL_LINES,
			KeyCode::ArrowRight => delta.x += KEY_SCROLL_LINES,
			KeyCode::ArrowLeft => delta.x -= KEY_SCROLL_LINES,
			KeyCode::PageDown => delta.y += PAGE_SCROLL_LINES,
			KeyCode::PageUp => delta.y -= PAGE_SCROLL_LINES,
			// Home/End jump to the top/bottom; the clamp settles the huge offset.
			KeyCode::Home => delta.y = i32::MIN / 2,
			KeyCode::End => delta.y = i32::MAX / 2,
			_ => {}
		}
	}
	if delta == IVec2::ZERO {
		return;
	}

	// resolve the container to scroll, in DOM priority order.
	let scrollable_ancestor = |start: Entity| {
		let mut current = Some(start);
		loop {
			let entity = current?;
			if scrolls.contains(entity) {
				break Some(entity);
			}
			// transclusion wins for *visual* ancestry: if a RenderRef holder renders
			// this entity in place, the holder (eg the page-host scrollport) is its
			// visual parent, even though its structural ChildOf points elsewhere (eg a
			// route entity under the router). Otherwise walk up ChildOf.
			current = refs
				.iter()
				.find(|(_, render_ref)| render_ref.target() == Some(entity))
				.map(|(holder, _)| holder)
				.or_else(|| parents.get(entity).ok().map(|child_of| child_of.parent()));
		}
	};
	let container = pointers
		.single()
		.ok()
		.and_then(|pointer| pointer.hover)
		.and_then(scrollable_ancestor)
		.or_else(|| focused.iter().next().and_then(scrollable_ancestor))
		// the page scrollport: the outermost ScrollPosition container reachable from
		// a buffer-root tree (the first one pre-order from a root). Generic over the
		// router, so beet_ui needs no dependency on it.
		.or_else(|| {
			roots.iter().find_map(|root| {
				tree.pre_order(root).into_iter().find(|entity| scrolls.contains(*entity))
			})
		});
	let Some(container) = container else { return };
	if let Ok(mut scroll) = scrolls.get_mut(container) {
		// the clamp_scroll_positions system settles this into range next frame.
		// saturating so the Home/End sentinel deltas can't overflow.
		let next = IVec2::new(
			scroll.offset.x.saturating_add(delta.x).max(0),
			scroll.offset.y.saturating_add(delta.y).max(0),
		);
		if next != scroll.offset {
			scroll.offset = next;
		}
	}
}

/// Convert a bevy cursor [`Vec2`] (cell-space, 1:1) to a signed cell.
pub(super) fn vec2_to_cell(position: Vec2) -> IVec2 {
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

	/// A full-width `display: block` interactive with a background (the sidebar
	/// row model) is hit across its whole width — clicking well past the end of
	/// its text still resolves to it, because the background fills the row with
	/// the element's painted cells.
	#[beet_core::test]
	fn full_width_block_is_hit_past_its_text() {
		let mut host = logging_host();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("row")
					.with_value(common_props::DisplayProp, Display::Block)
					.with_value(
						common_props::BackgroundColor,
						Color::srgb(0.2, 0.2, 0.2),
					),
			],
		);
		let row = host
			.app
			.world_mut()
			.spawn(rsx! { <div class="row">"Hi"</div> })
			.id();
		host.app.world_mut().entity_mut(host.host).add_child(row);
		host.step();
		// click far to the right of the 2-char text, still on the row's first line.
		host.send_input(&sgr(0, 30, 0, true));
		host.step();
		host.app
			.world()
			.resource::<PointerLog>()
			.down
			.contains(&row)
			.xpect_true();
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

	/// The maximum scroll offset across every container, on the given axis.
	fn max_offset(host: &mut TestHost, axis: impl Fn(IVec2) -> i32) -> i32 {
		host.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.map(|scroll| axis(scroll.offset))
			.max()
			.unwrap_or(0)
	}

	/// A keyboard scroll (ArrowDown, then PageDown) scrolls the page scrollport with
	/// nothing hovered: the fallback resolves the outermost buffer-root scroll
	/// container, like a browser scrolling the document.
	#[beet_core::test]
	fn key_scrolls_page_without_hover() {
		let mut host = TestHost::new();
		// the host itself is the page scrollport: a viewport-height scroll container
		// whose content overflows. No pointer ever moves, so nothing is hovered.
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("page")
					.with_value(common_props::Height, Length::Rem(4.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
			],
		);
		let body: String =
			(0..30).map(|i| format!("r{i}")).collect::<Vec<_>>().join("\n");
		host.spawn_content(rsx! { <div class="page"><pre>{body}</pre></div> });
		host.step();
		max_offset(&mut host, |offset| offset.y).xpect_eq(0);

		// ArrowDown (CSI B) with no hover: the page scrolls via the fallback.
		host.send_input(b"\x1b[B");
		host.step();
		let after_arrow = max_offset(&mut host, |offset| offset.y);
		(after_arrow > 0).xpect_true();

		// PageDown (CSI 6 ~) scrolls a whole page further.
		host.send_input(b"\x1b[6~");
		host.step();
		let after_page = max_offset(&mut host, |offset| offset.y);
		(after_page > after_arrow).xpect_true();

		// End jumps to the bottom (the clamp settles the huge offset to the max),
		// across every escape form a terminal may send it as.
		for end in [&b"\x1b[F"[..], &b"\x1bOF"[..], &b"\x1b[4~"[..]] {
			// reset to the top first so each form's jump is observable.
			host.send_input(b"\x1b[1~"); // Home
			host.step();
			max_offset(&mut host, |offset| offset.y).xpect_eq(0);
			host.send_input(end);
			host.step();
			let bottom = max_offset(&mut host, |offset| offset.y);
			(bottom > after_page).xpect_true();
			// Home returns to the top.
			host.send_input(b"\x1b[H");
			host.step();
			max_offset(&mut host, |offset| offset.y).xpect_eq(0);
		}
	}

	/// Hovering a link plays the hover tokens through the whole chain: the
	/// hit-test fires `PointerOver`, the `:hover` state re-resolves the cascade
	/// (the material `hover_dim` opacity), and a [`VisualTransition`] eases the
	/// displayed style toward the dimmed target over the motion-token duration.
	#[beet_core::test]
	fn link_hover_dim_animates() {
		let mut host = TestHost::new();
		host.app.add_plugins(
			crate::style::material::MaterialStylePlugin::default(),
		);
		// a long duration holds the transition mid-flight however slow the step
		host.app
			.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(vec![Rule::new()
				.with_selector(Selector::tag("a"))
				.with_value(
					common_props::TransitionDurationProp,
					Duration::from_secs(60),
				)]);
		host.spawn_content(rsx! { <div><a href="/x">"link"</a></div> });
		host.step();
		let link = host
			.app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(host.app.world())
			.find(|(_, element)| element.tag() == "a")
			.map(|(entity, _)| entity)
			.unwrap();
		let resting = host
			.app
			.world()
			.get::<VisualStyle>(link)
			.unwrap()
			.foreground;

		// hover the link's first glyph (any-motion event at cell 0,0)
		host.send_input(&sgr(35, 0, 0, true));
		host.step();
		host.step();

		// the hover state landed and re-resolved a dimmed target
		host.app
			.world()
			.get::<ElementStateMap>(link)
			.is_some_and(|map| map.contains(&ElementState::Hovered))
			.xpect_true();
		let target = host
			.app
			.world()
			.get::<VisualStyle>(link)
			.unwrap()
			.foreground;
		(target != resting).xpect_true();
		// the displayed style is easing toward it, not snapped
		let transition = host.app.world().get::<VisualTransition>(link).unwrap();
		transition.is_animating().xpect_true();
		(transition.current.foreground != target).xpect_true();
	}

	/// A backgroundless interactive (a text button) gains a visible hover: it
	/// fills with its surface at rest so the hover dim has something to darken,
	/// fixing the light-mode "no hover affordance" gap. The resolved hover
	/// background differs from (and is darker than) the resting fill.
	#[beet_core::test]
	fn backgroundless_button_hover_darkens_background() {
		use crate::style::material::classes;
		let mut host = TestHost::new();
		host.app.add_plugins(
			crate::style::material::MaterialStylePlugin::default(),
		);
		host.spawn_content(rsx! {
			<div><button {Classes::new([classes::BTN_TEXT])}>"Go"</button></div>
		});
		host.step();
		host.step();
		let button = host
			.app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(host.app.world())
			.find(|(_, element)| element.tag() == "button")
			.map(|(entity, _)| entity)
			.unwrap();
		let resting = host
			.app
			.world()
			.get::<VisualStyle>(button)
			.unwrap()
			.background
			.expect("text button fills with its surface at rest");

		// hover the button, settle the cascade + transition.
		let (col, row) = (1u32, 0u32);
		host.send_input(&sgr(35, col, row, true));
		for _ in 0..4 {
			host.step();
		}
		let hovered = host
			.app
			.world()
			.get::<VisualStyle>(button)
			.unwrap()
			.background
			.expect("hovered button keeps a background");
		// the dim darkened the fill: a different, lower-luminance colour.
		(hovered != resting).xpect_true();
		let luma = |color: Color| {
			let c = color.to_srgba();
			c.red + c.green + c.blue
		};
		(luma(hovered) < luma(resting)).xpect_true();
	}

	/// A horizontal wheel scrolls a wide container along its x axis (left then back
	/// right), proving every wheel direction is routed.
	#[beet_core::test]
	fn horizontal_wheel_scrolls_wide_container() {
		let mut host = TestHost::new();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("wide")
					.with_value(common_props::Width, Length::Rem(8.))
					.with_value(common_props::OverflowXProp, Overflow::Scroll),
			],
		);
		let wide: String = ('a'..='z').chain('A'..='Z').cycle().take(80).collect();
		host.spawn_content(rsx! { <div class="wide"><pre>{wide}</pre></div> });
		host.step();
		// hover the wide container, then wheel right (SGR button 67) to scroll x.
		host.send_input(&sgr(35, 1, 1, true));
		host.step();
		host.send_input(&sgr(67, 1, 1, true));
		host.step();
		let after_right = max_offset(&mut host, |offset| offset.x);
		(after_right > 0).xpect_true();
		// wheel left (SGR button 66) scrolls back toward the start.
		host.send_input(&sgr(66, 1, 1, true));
		host.step();
		(max_offset(&mut host, |offset| offset.x) < after_right).xpect_true();
	}
}
