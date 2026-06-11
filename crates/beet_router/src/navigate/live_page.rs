//! Live route rendering: paint the active route tree into a persistent
//! [`DoubleBuffer`] and re-render on navigation.
//!
//! The one-shot CLI path serializes a route's template tree to a string and
//! despawns it. The live TUI instead keeps the rendered tree alive and paints it
//! into a persistent [`DoubleBuffer`] each frame (via [`RealtimeParsePlugin`]),
//! re-rendering when the active [`CurrentPage`] changes. The injected
//! difference is exactly the buffer target plus the persistent lifecycle, not a
//! forked render path: the page tree is still built through the template
//! substrate, and the charcell pipeline still walks it (here by reference, via a
//! [`RenderRef`] slot under the buffer host).

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use bevy::math::UVec2;

/// A live-render host: a [`DoubleBuffer`] plus the [`RenderRef`] slot that
/// transcludes the active [`CurrentPage`] into it.
///
/// Spawn one with [`page_host`]. Mark a built route tree [`CurrentPage`]
/// and [`sync_current_page`] points the slot at it, so the charcell pipeline
/// paints it into the buffer. Navigating swaps `CurrentPage`, which re-points
/// the slot and repaints.
#[derive(Component)]
pub struct PageHost;

/// The slot entity (a child of the host) whose [`RenderRef`] transcludes the
/// current page. Kept distinct from the host so the host's buffer renders the
/// slot, and the slot's reference can be retargeted without touching the buffer.
#[derive(Component)]
pub struct PageSlot;

/// Spawn a live-render host: a `size`-cell [`DoubleBuffer`] whose content is a
/// viewport-filling `auto` scroll container holding the [`RenderRef`] slot that
/// transcludes the active [`CurrentPage`].
///
/// The scroll container is the page's scrollport (like the browser's scrollable
/// `<main>`): a page taller or wider than the viewport gets a scrollbar, a short
/// one does not. [`sync_current_page`] points the inner slot at the current page.
pub fn page_host(size: UVec2) -> impl Bundle {
	(
		PageHost,
		DoubleBuffer::new(size),
		children![(
			Element::new("div"),
			page_viewport_style(),
			children![(PageSlot, RenderRef::default())],
		)],
	)
}

/// The viewport-filling `overflow: auto` scroll container style for the page slot.
fn page_viewport_style() -> impl Bundle {
	inline_class![
		(style::common_props::OverflowXProp, style::Overflow::Auto),
		(style::common_props::OverflowYProp, style::Overflow::Auto),
		(style::common_props::Width, style::Length::ViewportWidth(100.)),
		(style::common_props::Height, style::Length::ViewportHeight(100.)),
	]
}

/// Registers the live-render sync system.
///
/// Pairs with [`CharcellPlugin`] + [`RealtimeParsePlugin`] (the repaint loop) and
/// [`NavigatorPlugin`] (which marks the navigated page [`CurrentPage`]).
#[derive(Default)]
pub struct LivePagePlugin;

impl Plugin for LivePagePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, sync_current_page);
	}
}

/// ECS system: point each host's [`RenderRef`] slot at the active
/// [`CurrentPage`], so the buffer paints the current route.
///
/// Runs when a new `CurrentPage` is added (navigation) and retargets the slot;
/// the next [`RealtimeParsePlugin`] repaint walks the new page through the
/// reference. A no-op when nothing changed.
pub fn sync_current_page(
	pages: Populated<Entity, Added<CurrentPage>>,
	mut slots: Query<&mut RenderRef, With<PageSlot>>,
) {
	let Some(page) = pages.iter().next() else {
		return;
	};
	for mut slot in slots.iter_mut() {
		slot.set_if_neq(RenderRef::new(page));
	}
}

/// Resolve `request` against the router's [`RouteTree`] and build the matched
/// scene route into a living entity tree, returning its root.
///
/// The live parallel of the static [`RenderRoot::render`] path: it shares the
/// route build *and* the ancestor layout middleware (header/sidebar/footer, the
/// document chrome) but forks at the output, handing back the built entity rather
/// than serializing and despawning it. That entity is kept alive to become a
/// [`CurrentPage`]. The static path is untouched.
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
	// build the route's own content (output `RenderRequest`), skipping the
	// `ExchangeAction` wrapper that would serialize then despawn the tree.
	let content = route.call::<Request, RenderRequest>(request).await?.0;
	// wrap it in the ancestor layout middleware (the `BaseLayout` document chrome),
	// transcluding the content by reference, exactly as `RenderRoot::render` does
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
	world.entity_mut(page).insert(DespawnAfterRender(vec![page]));
	Ok(page)
}

/// Make `page` the [`CurrentPage`], cleaning up the page it replaces.
///
/// Inserting [`CurrentPage`] fires [`single_current_page`], clearing the marker
/// from the outgoing page; that page's [`DespawnAfterRender`] ephemerals (a
/// per-request or parsed tree) are then despawned so pages do not accumulate. A
/// self-referential fixed route carries an empty set, so its entity survives.
///
/// The host slots are re-pointed at the new page *before* the despawn, so no
/// [`RenderRef`] references the outgoing tree when it is removed.
pub fn set_current_page(world: &mut World, page: Entity) {
	// snapshot the outgoing pages' cleanup sets before the marker moves.
	let mut pages = world
		.query_filtered::<(Entity, Option<&DespawnAfterRender>), With<CurrentPage>>();
	let stale: Vec<Entity> = pages
		.iter(world)
		.filter(|(entity, _)| *entity != page)
		.flat_map(|(_, despawn)| {
			despawn.map(|despawn| despawn.0.clone()).unwrap_or_default()
		})
		.filter(|entity| *entity != page)
		.collect();

	world.entity_mut(page).insert(CurrentPage);

	// re-point host slots now so nothing references the outgoing tree on despawn.
	let mut slots = world.query_filtered::<&mut RenderRef, With<PageSlot>>();
	for mut slot in slots.iter_mut(world) {
		slot.set_if_neq(RenderRef::new(page));
	}

	for entity in stale {
		if let Ok(entity) = world.get_entity_mut(entity) {
			entity.despawn();
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::math::UVec2;

	/// The live-TUI render stack minus the terminal host: charcell pipeline,
	/// per-frame repaint, the document chain, and the current-page sync.
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

	/// Build a page tree marked as the active page, returning its root entity.
	///
	/// Built through the template substrate (`spawn_template` + `Snippet`) so a
	/// page of `#[template]` widgets resolves its slots/lifecycle, exactly as the
	/// route constructors build per-request content.
	fn spawn_page(app: &mut App, bundle: impl Bundle) -> Entity {
		let page = app
			.world_mut()
			.spawn_template(Snippet::from_bundle(bundle))
			.unwrap()
			.id();
		app.world_mut().entity_mut(page).insert(CurrentPage);
		page
	}

	/// The host buffer's painted frame as plain text after one frame.
	fn frame(app: &mut App, host: Entity) -> String {
		// one frame: PreUpdate points the slot at CurrentPage, then the post-parse
		// pipeline paints the host buffer through the RenderRef slot.
		app.update();
		app.world()
			.get::<DoubleBuffer>(host)
			.unwrap()
			.current_buffer()
			.render_plain()
	}

	/// The active route renders into the persistent buffer, and navigating to a
	/// second route re-renders it (the previous page is dropped).
	#[beet_core::test]
	fn renders_and_re_renders_active_page() {
		let mut app = live_app();
		let host = app.world_mut().spawn(page_host(UVec2::new(40, 8))).id();

		// initial route: Alpha
		let alpha = spawn_page(&mut app, rsx! { <p>"Alpha page"</p> });
		frame(&mut app, host).xpect_contains("Alpha page");

		// navigate: a new page becomes current; the slot re-points and repaints.
		// the previous page leaves the active set (the single-page observer would
		// despawn it in the full app; here we drop it explicitly).
		app.world_mut().entity_mut(alpha).remove::<CurrentPage>();
		let _beta = spawn_page(&mut app, rsx! { <p>"Beta page"</p> });
		let out = frame(&mut app, host);
		out.as_str().xpect_contains("Beta page");
		out.xnot().xpect_contains("Alpha page");
	}

	/// The full live-navigation stack: a router + an in-world navigator + a page
	/// host. [`RouterPlugin`] brings the charcell/template/async plugins; the
	/// single-active-page invariant needs [`NavigatorPlugin`].
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
			.run_async_local(move |entity| {
				Navigator::navigate_to(entity, url)
			});
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
				render_action::fixed_route(
					"alpha",
					rsx! { <p>"Alpha page"</p> }
				),
				render_action::fixed_route(
					"beta",
					rsx! { <p>"Beta page"</p> }
				),
			]))
			.flush();
		let host = app.world_mut().spawn(page_host(UVec2::new(40, 8))).id();
		// home is `alpha`, so the on_add navigation paints the alpha route
		let nav =
			app.world_mut().spawn(Navigator::in_world(router, "alpha")).id();
		drive_until(&mut app, host, "Alpha page");

		// navigate to beta: the page swaps and repaints, alpha is gone
		navigate(&mut app, nav, "beta");
		drive_until(&mut app, host, "Beta page")
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
		set_current_page(app.world_mut(), page);
		frame(&mut app, host).xpect_contains("Hello");
	}
}
