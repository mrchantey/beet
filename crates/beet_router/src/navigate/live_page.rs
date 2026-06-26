//! Live route rendering: paint the active route tree into a persistent
//! [`DoubleBuffer`] and re-render on navigation.
//!
//! The one-shot CLI path serializes a route's template tree to a string and
//! despawns it. The live TUI instead keeps the rendered tree alive and paints it
//! into a persistent [`DoubleBuffer`] each frame (via [`RealtimeParsePlugin`]),
//! re-rendering when the surface's bound page changes. The injected
//! difference is exactly the buffer target plus the persistent lifecycle, not a
//! forked render path: the page tree is still built through the template
//! substrate, and the charcell pipeline still walks it (here by reference, via a
//! [`Portal`] slot under the buffer host).

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use bevy::math::UVec2;

/// A live-render host (a "surface"): a [`DoubleBuffer`] plus the [`Portal`] slot
/// that transcludes the page currently bound to this surface.
///
/// Spawn one with [`page_host`]. A [`Navigator`] co-located on this host calls
/// [`bind_surface_page`] to point the slot at a built page, so the charcell
/// pipeline paints it into the buffer. Navigating rebinds the slot and
/// repaints. Each surface is independent, so many can coexist (one per SSH
/// session) and show different pages at once.
#[derive(Component)]
pub struct PageHost;

/// The slot entity (a child of the host) whose [`Portal`] transcludes the
/// surface's bound page. Kept distinct from the host so the host's buffer renders
/// the slot, and the slot's reference can be retargeted without touching the buffer.
#[derive(Component)]
pub struct PageSlot;

/// Spawn a live-render host: a `size`-cell [`DoubleBuffer`] whose content is a
/// viewport-filling `auto` scroll container holding the [`Portal`] slot that
/// transcludes the surface's bound page.
///
/// The scroll container is the page's scrollport (like the browser's scrollable
/// `<main>`): a page taller or wider than the viewport gets a scrollbar, a short
/// one does not. [`bind_surface_page`] points the inner slot at the bound page.
pub fn page_host(size: UVec2) -> impl Bundle {
	(PageHost, DoubleBuffer::new(size), children![(
		Element::new("div"),
		page_viewport_style(),
		// the slot carries no `Portal` until a page is bound: absence is the
		// unresolved state, so `bind_surface_page` installs the reference.
		children![PageSlot],
	)])
}

/// The viewport-filling `overflow: auto` scroll container style for the page slot.
fn page_viewport_style() -> impl Bundle {
	inline_class![
		(style::common_props::OverflowXProp, style::Overflow::Auto),
		(style::common_props::OverflowYProp, style::Overflow::Auto),
		(
			style::common_props::Width,
			style::Length::ViewportWidth(100.)
		),
		(
			style::common_props::Height,
			style::Length::ViewportHeight(100.)
		),
	]
}

/// Marks an app that paints live navigator pages into [`page_host`] surfaces.
///
/// Pairs with [`CharcellPlugin`] + [`RealtimeParsePlugin`] (the repaint loop) and
/// [`NavigatorPlugin`] (which navigates). The page-to-surface binding is now
/// direct (a [`Navigator`] calls [`bind_surface_page`] on its co-located host),
/// so there is no per-frame sync system; this plugin remains the documented home
/// for the live-render composition.
#[derive(Default)]
pub struct LivePagePlugin;

impl Plugin for LivePagePlugin {
	fn build(&self, _app: &mut App) {}
}

/// Resolve `request` against the router's [`RouteTree`] and build the matched
/// scene route into a living entity tree, returning its root.
///
/// The live parallel of the static [`PageRoot::render`] path: it shares the
/// route build *and* the ancestor layout middleware (header/sidebar/footer, the
/// document chrome) but forks at the output, handing back the built entity rather
/// than serializing and despawning it. That entity is kept alive to be bound to a
/// surface via [`bind_surface_page`]. The static path is untouched.
pub async fn build_live_page(
	router: &AsyncEntity,
	mut request: Request,
) -> Result<Entity> {
	let path = request.path().clone();
	let router_id = router.id();
	// resolve the matched route node from the ancestor RouteTree (as Router does)
	let node = router
		.world()
		.with_state::<AncestorQuery<&RouteTree>, Result<Option<ActionNode>>>(
			move |query| {
				query
					.get(router_id)
					.map(|tree| tree.find(&path).cloned())
					.map_err(|_| {
						bevyhow!(
							"route tree not found, was the RouterPlugin added?"
						)
					})
			},
		)
		.await?;
	let Some(node) = node else {
		bevybail!("no route matched /{}", request.path_string());
	};
	// surface matched dynamic segments (`:id`) to the handler
	node.merge_path_params(&mut request);
	let parts = request.parts().clone();
	let route = router.world().entity(node.entity);
	// build the route's own content (output `PageRequest`), skipping the
	// `ExchangeOverload` adapter that would serialize then despawn the tree.
	let content = route.call::<Request, PageRequest>(request).await?.0;
	// wrap it in the ancestor layout middleware (the `BaseLayout` document chrome),
	// transcluding the content by reference, exactly as `PageRoot::render` does
	// for the static path; here the wrapped tree is kept alive as the page.
	route
		.call_with_middleware::<RequestParts, Entity>(
			Action::new_fixed(content),
			parts,
		)
		.await
}

/// Parse fetched [`MediaBytes`] (markdown/html) into a living entity tree on a
/// fresh entity, returning its root.
///
/// The remote/HTTP counterpart of [`build_live_page`]: a network fetch yields
/// bytes that must become a tree, so they parse through the same template
/// substrate the route build uses (via [`MediaParser`]). The tree is marked for
/// cleanup on the next page swap.
pub fn parse_page(world: &mut World, bytes: MediaBytes) -> Result<Entity> {
	let mut entity = world.spawn_empty();
	MediaParser::new().parse(ParseContext::new(&mut entity, &bytes))?;
	let page = entity.id();
	// the parsed tree is this page's own; clean it up when the page is replaced.
	world
		.entity_mut(page)
		.insert(DespawnAfterRender(vec![page]));
	Ok(page)
}

/// Bind `page` to `host` (a [`PageHost`] surface), cleaning up the page it
/// replaces.
///
/// The host's [`PageSlot`] [`Portal`] is re-pointed at `page` *before* the
/// despawn, so nothing references the outgoing tree when it is removed. The
/// outgoing page is the host's current [`RenderSurfaceOf`] (the one-to-one
/// back-link `RenderSurface(host)` maintains); its [`DespawnAfterRender`]
/// ephemerals (a per-request or parsed tree) are then despawned so pages do not
/// accumulate, while a self-referential fixed route (empty set) survives.
///
/// Scoped to one surface, so binding a page on one SSH session never disturbs
/// another session's page.
pub fn bind_surface_page(world: &mut World, host: Entity, page: Entity) {
	let Some(slot) = page_slot_of(world, host) else {
		error!("page host {host} has no PageSlot child");
		return;
	};
	// the page currently bound to this surface, to clean up after the swap.
	let outgoing = world
		.entity(host)
		.get::<RenderSurfaceOf>()
		.map(|surface| surface.page());

	// back-link the page to its surface (one-to-one, so the outgoing page's link is
	// dropped), then re-point the slot at it.
	world.entity_mut(page).insert(RenderSurface(host));
	world.entity_mut(slot).insert(Portal::new(page));

	// despawn the outgoing page's ephemerals now that nothing references them.
	if let Some(outgoing) = outgoing.filter(|outgoing| *outgoing != page) {
		let stale = world
			.entity(outgoing)
			.get::<DespawnAfterRender>()
			.map(|despawn| despawn.0.clone())
			.unwrap_or_default();
		for entity in stale.into_iter().filter(|entity| *entity != page) {
			if let Ok(entity) = world.get_entity_mut(entity) {
				entity.despawn();
			}
		}
	}
}

impl PageHost {
	/// The [`PageHost`] surface a navigator drives: the nearest self-or-ancestor
	/// host, walking `ChildOf`.
	///
	/// A navigator is co-located on (or nested under) its host, so which surface
	/// it drives is structural rather than a nullable field. `None` only for a
	/// navigator with no host (a misconfiguration the caller logs).
	pub fn of(world: &World, entity: Entity) -> Option<Entity> {
		let mut current = entity;
		loop {
			let entity_ref = world.get_entity(current).ok()?;
			if entity_ref.contains::<PageHost>() {
				return Some(current);
			}
			match entity_ref.get::<ChildOf>() {
				Some(child_of) => current = child_of.parent(),
				None => return None,
			}
		}
	}
}

/// The [`PageSlot`] descendant of `host` (a grandchild as [`page_host`] spawns it).
fn page_slot_of(world: &World, host: Entity) -> Option<Entity> {
	let mut stack = vec![host];
	while let Some(entity) = stack.pop() {
		if world.entity(entity).contains::<PageSlot>() {
			return Some(entity);
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			stack.extend(children.iter());
		}
	}
	None
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::math::UVec2;

	/// The live-TUI render stack minus the terminal host: charcell pipeline,
	/// per-frame repaint, and the document chain.
	fn live_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TemplatePlugin,
			DocumentPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
		));
		app
	}

	/// Build a page tree bound to `host`'s surface, returning its root entity.
	///
	/// Built through the template substrate (`spawn_template` + `Snippet`) so a
	/// page of `#[template]` widgets resolves its slots/lifecycle, exactly as the
	/// route constructors build per-request content.
	fn spawn_page(app: &mut App, host: Entity, bundle: impl Bundle) -> Entity {
		let page = app
			.world_mut()
			.spawn_template(Snippet::from_bundle(bundle))
			.unwrap()
			.id();
		bind_surface_page(app.world_mut(), host, page);
		page
	}

	/// The host buffer's painted frame as plain text after one frame.
	fn frame(app: &mut App, host: Entity) -> String {
		// one frame: the post-parse pipeline paints the host buffer through the
		// Portal slot bound by `bind_surface_page`.
		app.update();
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.current_buffer()
			.render_plain()
	}

	/// The bound page renders into the persistent buffer, and binding a second page
	/// re-renders it (the previous page is dropped).
	#[beet_core::test]
	fn renders_and_re_renders_active_page() {
		let mut app = live_app();
		let host = app.world_mut().spawn(page_host(UVec2::new(40, 8))).id();

		// initial page: Alpha
		spawn_page(&mut app, host, rsx! { <p>"Alpha page"</p> });
		frame(&mut app, host).xpect_contains("Alpha page");

		// rebind: a new page takes the surface; the slot re-points and repaints,
		// the previous page is unbound (and despawned if ephemeral).
		spawn_page(&mut app, host, rsx! { <p>"Beta page"</p> });
		let out = frame(&mut app, host);
		out.as_str().xpect_contains("Beta page");
		out.xnot().xpect_contains("Alpha page");
	}

	/// The full live-navigation stack: a router + an in-world navigator + a page
	/// host. [`RouterPlugin`] brings the charcell/template/async plugins;
	/// [`NavigatorPlugin`] brings link handling and history shortcuts.
	fn nav_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
			NavigatorPlugin,
		));
		app
	}

	/// Queue an in-world navigation to `path` on the navigator entity.
	fn navigate(app: &mut App, nav: Entity, path: &str) {
		let url = Url::parse(path);
		app.world_mut()
			.entity_mut(nav)
			.run_async_local(move |entity| Navigator::navigate_to(entity, url));
	}

	/// Drive the app until the host frame contains `needle`, returning the frame.
	fn drive_until(app: &mut App, host: Entity, needle: &str) -> String {
		for _ in 0..200 {
			let frame = frame(app, host);
			if frame.contains(needle) {
				return frame;
			}
		}
		panic!("host frame never contained '{needle}'");
	}

	/// An in-world navigation lands the resolved route as the current page and
	/// paints it; navigating again repaints with the new route, not the old.
	#[beet_core::test]
	async fn navigates_live_pages_in_world() {
		let mut app = nav_app();
		let router = app
			.world_mut()
			.spawn((Router, children![
				render_action::fixed_func_route(
					"alpha",
					|| rsx! { <p>"Alpha page"</p> }
				),
				render_action::fixed_func_route(
					"beta",
					|| rsx! { <p>"Beta page"</p> }
				),
			]))
			.flush();
		// the navigator is co-located on its page host (one surface)
		let host = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "alpha"),
			))
			.id();
		// home is `alpha`, so the on_add navigation paints the alpha route
		drive_until(&mut app, host, "Alpha page");

		// navigate to beta: the page swaps and repaints, alpha is gone
		navigate(&mut app, host, "beta");
		drive_until(&mut app, host, "Beta page")
			.xnot()
			.xpect_contains("Alpha page");
	}

	/// A router whose pages cross-link, for the link-click navigation tests.
	fn linked_router(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((Router, children![
				render_action::fixed_func_route("alpha", || {
					rsx! { <a href="/beta">"to beta"</a> }
				}),
				render_action::fixed_func_route("beta", || {
					rsx! { <p>"Beta page"</p> }
				}),
			]))
			.flush()
	}

	/// The `<a>` element entity within `host`'s currently bound page.
	fn link_in(app: &mut App, host: Entity) -> Entity {
		let page = app
			.world()
			.get::<RenderSurfaceOf>(host)
			.expect("host has a bound page")
			.page();
		// the <a> is the page's descendant Element tagged "a".
		let descendants = std::iter::successors(Some(vec![page]), |level| {
			let next = level
				.iter()
				.filter_map(|entity| app.world().get::<Children>(*entity))
				.flat_map(|children| children.iter())
				.collect::<Vec<_>>();
			(!next.is_empty()).then_some(next)
		})
		.flatten();
		descendants
			.filter(|entity| {
				app.world()
					.get::<Element>(*entity)
					.is_some_and(|element| element.tag() == "a")
			})
			.next()
			.expect("page has an <a> element")
	}

	/// Clicking an internal link navigates the clicked link's own surface, even when
	/// many surfaces coexist: the regression for `on_link_click` resolving the
	/// navigator from the link's surface rather than assuming a single global one
	/// (the prior `navigators.single()` returned `Err` with 2+ sessions, so no
	/// session navigated). One host clicks through to beta; the other stays on alpha.
	#[beet_core::test]
	async fn link_click_navigates_only_its_surface() {
		let mut app = nav_app();
		let router = linked_router(&mut app);
		let first = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "alpha"),
			))
			.id();
		let second = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "alpha"),
			))
			.id();
		drive_until(&mut app, first, "to beta");
		drive_until(&mut app, second, "to beta");

		// click the first host's link (as the hit-test would on a real click).
		let link = link_in(&mut app, first);
		app.world_mut()
			.entity_mut(link)
			.trigger(PointerUp::new(link));
		drive_until(&mut app, first, "Beta page");

		// the second host never navigated: still on alpha, never beta.
		frame(&mut app, second)
			.xpect_contains("to beta")
			.xnot()
			.xpect_contains("Beta page");
	}

	/// A document layout that transcludes the route content into its `<main>` slot
	/// by reference (a [`Portal`]), the shape every live page has once wrapped in
	/// [`BaseLayout`]. The chrome carries no link of its own.
	#[template]
	fn SlotLayout() -> impl Bundle {
		rsx! { <body><main><Slot/></main></body> }
	}

	/// The first `<a>` element anywhere in the world (these tests spawn one link).
	/// Unlike [`link_in`] it does not walk `Children`, so it finds a link inside
	/// Portal-transcluded content that no `ChildOf` path reaches.
	fn any_link(app: &mut App) -> Entity {
		app.world_mut()
			.query::<(Entity, &Element)>()
			.iter(app.world())
			.find(|(_, element)| element.tag() == "a")
			.map(|(entity, _)| entity)
			.expect("a link element exists")
	}

	/// Clicking a link inside Portal-transcluded layout content navigates. The
	/// route content is layouted into the slot *by reference*, so the link has no
	/// `ChildOf` path to the page root's [`RenderSurface`]; resolving the navigator
	/// must cross the transclusion. Regression for the markdown-link bug — every
	/// in-page link sat in such content, so a click resolved no surface and did
	/// nothing, while the sidebar (a real `ChildOf` descendant of the page root)
	/// worked.
	#[beet_core::test]
	async fn link_in_layouted_content_navigates() {
		let mut app = nav_app();
		let router = app
			.world_mut()
			.spawn((Router, BaseLayout::<SlotLayout>::default(), children![
				render_action::fixed_func_route("alpha", || {
					rsx! { <p>"go "<a href="/beta">"to beta"</a>" now"</p> }
				}),
				render_action::fixed_func_route("beta", || {
					rsx! { <p>"Beta page"</p> }
				}),
			]))
			.flush();
		let host = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "alpha"),
			))
			.id();
		drive_until(&mut app, host, "to beta");

		// click the transcluded link (as the hit-test would, via PointerUp on it).
		let link = any_link(&mut app);
		app.world_mut()
			.entity_mut(link)
			.trigger(PointerUp::new(link));
		drive_until(&mut app, host, "Beta page");
	}

	/// Whether `host`'s page slot has been bound to a page.
	fn slot_bound(app: &App, host: Entity) -> bool {
		page_slot_of(app.world(), host)
			.and_then(|slot| app.world().entity(slot).get::<Portal>())
			.is_some()
	}

	/// The default navigator's `about:blank` home renders an empty page in-place
	/// without a network fetch (regression: it used to HTTP-fetch `about:blank`,
	/// fail to parse, and panic the async task).
	#[beet_core::test]
	async fn default_home_renders_blank() {
		let mut app = nav_app();
		// the navigator is co-located on its page host
		let host = app
			.world_mut()
			.spawn((page_host(UVec2::new(40, 8)), Navigator::default()))
			.id();
		// drive the async on_add navigation until the surface slot is bound
		for _ in 0..200 {
			frame(&mut app, host);
			if slot_bound(&app, host) {
				break;
			}
		}
		slot_bound(&app, host).xpect_true();
	}

	/// Two surfaces render independently: each navigator binds only its own host's
	/// slot, so two sessions show different pages at once (the multi-tenant
	/// invariant the SSH TUI server relies on).
	#[beet_core::test]
	async fn two_surfaces_render_independently() {
		let mut app = nav_app();
		let router = app
			.world_mut()
			.spawn((Router, children![
				render_action::fixed_func_route("alpha", || {
					rsx! { <p>"Alpha page"</p> }
				}),
				render_action::fixed_func_route("beta", || {
					rsx! { <p>"Beta page"</p> }
				}),
			]))
			.flush();
		// each surface's navigator is co-located on its own host
		let host_a = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "alpha"),
			))
			.id();
		let host_b = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "beta"),
			))
			.id();

		// drive until both surfaces have painted their own route
		for _ in 0..400 {
			let frame_a = frame(&mut app, host_a);
			let frame_b = frame(&mut app, host_b);
			if frame_a.contains("Alpha page") && frame_b.contains("Beta page") {
				break;
			}
		}
		frame(&mut app, host_a)
			.xpect_contains("Alpha page")
			.xnot()
			.xpect_contains("Beta page");
		frame(&mut app, host_b)
			.xpect_contains("Beta page")
			.xnot()
			.xpect_contains("Alpha page");
	}

	/// The `MediaBytes` → living tree primitive: parsed markdown becomes a page
	/// that the host paints.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn parse_primitive_paints() {
		let mut app = live_app();
		let host = app.world_mut().spawn(page_host(UVec2::new(40, 8))).id();
		let bytes = MediaBytes::new_markdown("# Hello");
		let page = parse_page(app.world_mut(), bytes).unwrap();
		bind_surface_page(app.world_mut(), host, page);
		frame(&mut app, host).xpect_contains("Hello");
	}
}
