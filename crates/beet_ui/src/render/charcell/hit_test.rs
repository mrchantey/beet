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
	/// The topmost entity at `cell` within `surface`'s buffer only, so a cursor on
	/// one surface never resolves to another surface's content (one per SSH session).
	///
	/// Prefers the entity that actually painted the cell (recorded on the painted
	/// [`Cell`]), which precisely accounts for inline flow (so a click on inline
	/// `<a>` text lands on the link), stacking, clipping, and the scroll transform.
	/// Falls back to a geometric rect walk for cells no glyph painted (eg a
	/// transparent container's padding).
	fn entity_at_surface(&self, surface: Entity, cell: IVec2) -> Option<Entity> {
		let Ok((root, buffer)) = self.roots.get(surface) else {
			return None;
		};
		// the precisely-painted entity at this cell, if any glyph painted it.
		if cell.x >= 0 && cell.y >= 0 {
			let pos = UVec2::new(cell.x as u32, cell.y as u32);
			if let Some(entity) =
				buffer.front_buffer().get(pos).and_then(|cell| cell.entity)
			{
				return Some(entity);
			}
		}
		// geometric rect walk for cells no glyph painted.
		let viewport = buffer.current_buffer().size();
		let ordered = self.tree.pre_order(root);
		let contexts =
			resolve_contexts(root, &ordered, &self.charcell, &self.tree, viewport);
		let painted =
			stacking_order(root, &self.charcell, &self.tree, &managed_set(
				root,
				&self.charcell,
				&self.tree,
			));
		// topmost wins: scan the back-to-front order in reverse.
		painted
			.iter()
			.rev()
			.find(|&&entity| hit(entity, cell, &contexts, &self.charcell))
			.copied()
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
	mut pointers: Query<&mut Pointer>,
	mut last_cursor: Local<HashMap<Entity, IVec2>>,
) -> Result {
	// the pointer lives on the surface (window) entity, so cursor/button events
	// route to their own surface's pointer and hit-test only that surface's buffer.
	for moved in cursor.read() {
		let surface = moved.window;
		let cell = vec2_to_cell(moved.position);
		last_cursor.insert(surface, cell);
		let Ok(mut hover) = pointers.get_mut(surface) else {
			continue;
		};
		let target = hit_test.entity_at_surface(surface, cell);
		match (hover.hover, target) {
			(Some(old), Some(new)) if old != new => {
				commands.entity(old).trigger(PointerOut::new(surface));
				commands.entity(new).trigger(PointerOver::new(surface));
				hover.hover = Some(new);
			}
			(None, Some(new)) => {
				commands.entity(new).trigger(PointerOver::new(surface));
				hover.hover = Some(new);
			}
			(Some(old), None) => {
				commands.entity(old).trigger(PointerOut::new(surface));
				hover.hover = None;
			}
			_ => {}
		}
	}

	// button presses target the entity under that surface's most recent cursor cell.
	for button in buttons.read() {
		let surface = button.window;
		let Some(&cell) = last_cursor.get(&surface) else {
			continue;
		};
		let Some(target) = hit_test.entity_at_surface(surface, cell) else {
			continue;
		};
		match button.state {
			ButtonState::Pressed => {
				commands.entity(target).trigger(PointerDown::new(surface));
			}
			ButtonState::Released => {
				commands.entity(target).trigger(PointerUp::new(surface));
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
	pointers: Query<&Pointer>,
	focused: Query<Entity, With<Focus>>,
	parents: Query<&ChildOf>,
	// transclusion: a `Portal` holder is the charcell parent of the entity it
	// points at, so the ancestor walk can cross from transcluded content (eg a
	// page) up into the holder's container (eg the page-host scrollport).
	refs: Query<&PortalOf>,
	tree: CharcellTree,
	roots: Query<(Entity, &DoubleBuffer)>,
	// `CharcellQuery` (p0) reads `ScrollPosition`, so it can't coexist with the
	// `&mut ScrollPosition` writer (p1) outside a `ParamSet`.
	mut params: ParamSet<(CharcellQuery, Query<&mut ScrollPosition>)>,
) {
	// accumulate this frame's scroll delta per surface (window), in cells, so each
	// session scrolls only its own page.
	let mut deltas = HashMap::<Entity, IVec2>::default();
	for ev in wheel.read() {
		let lines = match ev.unit {
			MouseScrollUnit::Line => MOUSE_SCROLL_LINES,
			// pixel deltas are coarse here; treat each as one notch
			MouseScrollUnit::Pixel => MOUSE_SCROLL_LINES,
		};
		let delta = deltas.entry(ev.window).or_default();
		// wheel y is positive up (content scrolls up), so the offset moves opposite;
		// wheel x is positive right (content scrolls right), so the offset follows it.
		delta.x += ev.x.signum() as i32 * lines * (ev.x != 0.) as i32;
		delta.y -= ev.y.signum() as i32 * lines * (ev.y != 0.) as i32;
	}
	// group this frame's pressed keys by their source surface.
	let mut keys_by_surface = HashMap::<Entity, Vec<KeyCode>>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		keys_by_surface.entry(key.window).or_default().push(key.key_code);
	}
	for (surface, pressed) in &keys_by_surface {
		// alt+arrows are reserved for history nav (back/forward), not scrolling.
		let alt = pressed
			.iter()
			.any(|key| matches!(key, KeyCode::AltLeft | KeyCode::AltRight));
		let delta = deltas.entry(*surface).or_default();
		for key in pressed {
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
	}

	// resolve and apply the scroll for each surface independently.
	for (surface, delta) in deltas {
		if delta == IVec2::ZERO {
			continue;
		}
		// the surface's own buffer root and viewport, so the box model resolves the
		// same way layout did and the scrollable-extent check matches the geometry.
		let Ok((root, buffer)) = roots.get(surface) else {
			continue;
		};
		let viewport = buffer.size();

		// resolve the container to scroll, in DOM priority order, scoped to this
		// surface, skipping any container that can't scroll on the delta's axis — so
		// the wheel falls through a pinned/zero-extent inner container to the real
		// scrollport beneath it, like a browser.
		let container = {
			let charcell = params.p0();
			// whether `entity` is a scroll container that can move on the delta axis.
			let can_scroll = |entity: Entity| {
				charcell
					.unresolved_node(entity)
					.ok()
					.filter(|node| node.is_scroll_container())
					.map(|node| scroll_state(&node, &charcell, viewport).max_offset())
					.is_some_and(|max| {
						(delta.y != 0 && max.y > 0) || (delta.x != 0 && max.x > 0)
					})
			};
			// the nearest ancestor (self-inclusive) that can scroll the delta axis.
			let scrollable_ancestor = |start: Entity| {
				let mut current = Some(start);
				loop {
					let entity = current?;
					if can_scroll(entity) {
						break Some(entity);
					}
					// transclusion wins for *visual* ancestry: a Portal holder is the
					// visual parent of the entity it renders in place; else walk ChildOf.
					current = refs
						.get(entity)
						.ok()
						.and_then(|render_ref_of| render_ref_of.holders().first().copied())
						.or_else(|| {
							parents.get(entity).ok().map(|child_of| child_of.parent())
						});
				}
			};
			pointers
				.get(surface)
				.ok()
				.and_then(|pointer| pointer.hover)
				.and_then(scrollable_ancestor)
				.or_else(|| focused.iter().next().and_then(scrollable_ancestor))
				// the page scrollport: the outermost scrollable container reachable
				// from this surface's buffer-root tree (first scrollable pre-order).
				.or_else(|| {
					tree.pre_order(root).into_iter().find(|entity| can_scroll(*entity))
				})
		};
		let Some(container) = container else { continue };
		if let Ok(mut scroll) = params.p1().get_mut(container) {
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

	/// A full-width `display: block` interactive *without* a background (the
	/// sidebar row model) is hit across its whole width — clicking well past the
	/// end of its text still resolves to it via the geometric rect fallback, so
	/// the row needs no fill to be fully clickable.
	#[beet_core::test]
	fn full_width_block_is_hit_past_its_text() {
		let mut host = logging_host();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				Rule::class("row")
					.with_value(common_props::DisplayProp, Display::Block),
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
		// hover now tracks the second row's entity (or its text child); the pointer
		// lives on the host surface (window) entity itself.
		let hover = host
			.app
			.world()
			.entity(host.host)
			.get::<Pointer>()
			.unwrap()
			.hover;
		hover.xpect_some();
	}

	/// Two surfaces hit-test independently: a click routed to surface A resolves to
	/// A's content and never B's (the multi-tenant pointer invariant the SSH TUI
	/// server relies on, since both surfaces share the 0-indexed cell grid).
	#[beet_core::test]
	fn click_routes_to_its_own_surface() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellTuiPlugin));
		app.init_resource::<PointerLog>();
		app.add_observer(|ev: On<PointerDown>, mut log: ResMut<PointerLog>| {
			log.down.push(ev.event_target());
		});
		// spawn an independent surface (channel terminal + buffer) holding `label`.
		let spawn_surface = |app: &mut App, label: &str| -> (Entity, Entity) {
			let (channel, terminal) =
				ChannelTerminal::new(TerminalConfig::default());
			let content = app.world_mut().spawn(rsx! { <div>{label}</div> }).id();
			let host = app
				.world_mut()
				.spawn((channel, terminal, DoubleBuffer::new(UVec2::new(20, 4))))
				.id();
			app.world_mut().entity_mut(host).add_child(content);
			(host, content)
		};
		let (host_a, content_a) = spawn_surface(&mut app, "AAAA");
		let (_host_b, content_b) = spawn_surface(&mut app, "BBBB");
		app.update();

		// click cell (0,0) on surface A's channel only
		app.world_mut()
			.get_mut::<ChannelTerminal>(host_a)
			.unwrap()
			.send_input(&sgr(0, 0, 0, true))
			.unwrap();
		app.update();

		let log = app.world().resource::<PointerLog>();
		log.down.contains(&content_a).xpect_true();
		log.down.contains(&content_b).xpect_false();
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

	/// The resolved background of the `.btn-text` button under `scheme` after the
	/// pointer has hovered it (the cascade + transition settled).
	fn hovered_button_background(scheme: ClassName) -> Option<Color> {
		use crate::style::material::classes;
		let mut host = TestHost::new();
		host.app.add_plugins(
			crate::style::material::MaterialStylePlugin::default(),
		);
		host.spawn_content(rsx! {
			<div {Classes::new([scheme])}>
				<button {Classes::new([classes::BTN_TEXT])}>"Go"</button>
			</div>
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
		// no container at rest.
		host.app.world().get::<VisualStyle>(button).unwrap().background.xpect_none();
		// hover the button, settle the cascade + transition.
		host.send_input(&sgr(35, 1, 0, true));
		for _ in 0..4 {
			host.step();
		}
		host.app.world().get::<VisualStyle>(button).unwrap().background
	}

	/// A container-less interactive (text button) gains a *visible fill* on hover
	/// in the light scheme — the fix for the light-mode "no hover affordance"
	/// gap — but no fill in the dark scheme, where the hover reads as a text dim
	/// instead. Neither carries a background at rest.
	#[beet_core::test]
	fn backgroundless_button_hover_is_scheme_aware() {
		use crate::style::material::classes;
		// light: a hover fill appears.
		hovered_button_background(classes::LIGHT_SCHEME).xpect_some();
		// dark: no hover fill (the HoverSurface token is unset there).
		hovered_button_background(classes::DARK_SCHEME).xpect_none();
	}

	/// A wheel over content nested in a scroll container that *can't* scroll
	/// (its content fits) falls through to the outer scrollport that can, like a
	/// browser — so a pinned/zero-extent inner container never swallows the wheel.
	#[beet_core::test]
	fn wheel_falls_through_non_scrollable_inner_container() {
		let mut host = TestHost::new();
		host.app.world_mut().get_resource_or_init::<RuleSet>().extend_rules(
			vec![
				// the outer page: short, scrollable, its content overflows.
				Rule::class("outer")
					.with_value(common_props::Height, Length::Rem(4.))
					.with_value(common_props::OverflowYProp, Overflow::Scroll),
				// the inner box: tall enough to hold its content, also a scroll
				// container, but its own content fits so it can't scroll.
				Rule::class("inner")
					.with_value(common_props::Height, Length::Rem(20.))
					.with_value(common_props::OverflowYProp, Overflow::Auto),
			],
		);
		let content = host
			.app
			.world_mut()
			.spawn(rsx! {
				<div class="outer">
					<div class="inner"><pre>"a\nb\nc"</pre></div>
					<pre>"d\ne\nf\ng\nh\ni\nj\nk"</pre>
				</div>
			})
			.id();
		host.app.world_mut().entity_mut(host.host).add_child(content);
		host.step();
		// hover the inner box (top-left), then wheel down.
		host.send_input(&sgr(35, 1, 1, true));
		host.step();
		host.send_input(&sgr(65, 1, 1, true));
		host.step();
		// the OUTER scrolled (the inner can't), so some container has a non-zero
		// offset that belongs to the outer page scrollport.
		let scrolled = host
			.app
			.world_mut()
			.query::<&ScrollPosition>()
			.iter(host.app.world())
			.any(|scroll| scroll.offset.y > 0);
		scrolled.xpect_true();
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
