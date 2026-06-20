//! Layout render middleware: wrap a route's rendered content in a document
//! layout (the web `<html>`/`<head>` document, an article/sidebar layout, etc.)
//! without reparenting or re-resolving it.
//!
//! [`BaseLayout`] is a render-middleware component (registered like any other
//! middleware, eg [`RequestLogger`]). For every descendant render route it runs
//! the inner handler to obtain the content render root, then builds the layout,
//! an ordinary `#[template]` widget, with the content routed into its default
//! `<Slot>` as a [`Portal`] transclusion. The content is rendered *in place,
//! by reference*: it is never reparented under the layout nor re-resolved, so a
//! persistent fixed route survives request after request.
//!
//! The layout wraps **every** request regardless of target. Non-visual document
//! chrome (`<head>`/`<style>`/`<script>`) simply does not paint in the terminal
//! (it resolves to `display: none`; see the user-agent style layer), so the same
//! layout renders correctly on web and terminal.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Render middleware wrapping every descendant render route in the document
/// layout widget `C` — an ordinary `#[template]` widget with a default `<Slot>`.
///
/// Add it to an ancestor of the routes it should wrap (eg the router entity),
/// exactly like any other middleware ([`RequestLogger`], [`HelpHandler`]):
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// #[template]
/// fn PageLayout() -> impl Bundle { rsx! { <html><body><Slot/></body></html> } }
/// let bundle = (Router, BaseLayout::<PageLayout>::default());
/// ```
///
/// For each request it runs the inner route to obtain the content render root,
/// then builds `C` with that content routed into its default `<Slot>` (a
/// [`Portal`] transclusion).
#[action]
#[derive(Component)]
#[component(on_add = on_add_middleware::<Self, RequestParts, Entity>)]
pub async fn BaseLayout<C>(
	cx: ActionContext<(RequestParts, Next<RequestParts, Entity>)>,
) -> Result<Entity>
where
	C: 'static + Send + Sync + Clone + Default + BuildTemplate,
{
	let (parts, next) = &cx.input;
	// resolve the inner content render root, then wrap it
	let content = next.call(parts.clone()).await?;
	// the request parts feed the render context (active nav, per-route meta, etc.)
	let parts = parts.clone();
	// the middleware runs with the matched route as caller, the reliable in-tree
	// anchor for tree-scoped widgets (the rendered content may be detached)
	let route = cx.id();
	next.world()
		.clone()
		.with(move |world: &mut World| {
			wrap_content::<C>(world, parts, route, content)
		})
		.await
}

/// Spawn the layout `C` around the existing `content` render root, returning the
/// layout as the new render root.
///
/// The content is routed into the layout's default `<Slot>` as a [`SlotChild`]
/// carrying a [`Portal`]: the walker splices it at the layout's slot,
/// transcluding the existing content entity **by reference**. The layout subtree
/// is ephemeral and despawned after render (along with the content's own
/// ephemerals), but the referenced content is never owned or despawned here, so
/// a persistent fixed route survives request after request.
fn wrap_content<C: 'static + Send + Sync + Clone + Default + BuildTemplate>(
	world: &mut World,
	parts: RequestParts,
	route: Entity,
	content: Entity,
) -> Result<Entity> {
	wrap_content_with(world, parts, route, content, |world, rendered| {
		world
			.spawn_template(Snippet::from_bundle((
				C::default().into_snippet_bundle(),
				children![(Portal::new(rendered), SlotChild::new())],
			)))
			.map(|entity| entity.id())
	})
}

/// The shared wrap step of every layout middleware: read the inner render root,
/// push the request-scoped [`RequestContext`] onto the [`RequestContextStack`]
/// around `build_layout` (which spawns the layout with the content transcluded
/// into its default slot, returning the layout entity), then mark the layout as
/// the new render root.
///
/// The layout subtree is ephemeral and despawned after render (along with the
/// content's own ephemerals), but the referenced content is never owned or
/// despawned here, so a persistent fixed route survives request after request.
pub(crate) fn wrap_content_with(
	world: &mut World,
	parts: RequestParts,
	route: Entity,
	content: Entity,
	build_layout: impl FnOnce(&mut World, Entity) -> Result<Entity>,
) -> Result<Entity> {
	// the inner render root names the entity to render and its ephemerals
	let (rendered, content_despawn) = {
		let entity = world.entity(content);
		let rendered = entity
			.get::<PageRoot>()
			.ok_or_else(|| {
				bevyhow!("layout inner handler did not yield a render root")
			})?
			.rendered();
		let despawn = entity
			.get::<DespawnAfterRender>()
			.map(|despawn| despawn.0.clone())
			.unwrap_or_default();
		(rendered, despawn)
	};

	// resolve the entity owning this request's route tree once, as the nearest
	// tree-bearing ancestor of the in-tree route anchor (`route` is always in the
	// served tree, where `rendered` content may be detached). Threading this
	// handle lets tree-scoped widgets read the tree with an O(1) get instead of
	// re-walking each render; falling back to `route` when no tree ancestor exists
	// (eg a synthetic test root) leaves such a widget's lookup empty, not wrong.
	let router = world
		.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
			trees.get_entity(route)
		})
		.unwrap_or(route);

	// the request-scoped render context, read by the layout's scene systems: the
	// request parts, the rendered content entity (off which widgets query
	// per-route components, eg `ArticleMeta` parsed from frontmatter), the matched
	// route entity, and its tree-owning `router`. Pushed onto the stack for the
	// synchronous layout build, then popped — a stack so a nested render restores
	// this context on completion.
	world
		.resource_mut::<RequestContextStack>()
		.push(RequestContext::new(parts, rendered, route, router));
	let layout_result = build_layout(world, rendered);
	world.resource_mut::<RequestContextStack>().pop();
	let layout = layout_result?;

	// link the layout root to the transcluded content, distinct from the
	// self-referential render root: a layout-head `@entity:PageRoot::` binding
	// follows this to read the route's `ArticleMeta` across the transclusion.
	world
		.entity_mut(layout)
		.insert(LayoutContent::new(rendered));

	// despawn the layout subtree plus the content's ephemerals after render
	let mut to_despawn = vec![layout];
	to_despawn.extend(content_despawn);
	PageRoot::insert(&mut world.entity_mut(layout), to_despawn);
	layout.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	// the `MetaLayout` test template's site-title link, gated like its only user.
	#[cfg(feature = "json")]
	use beet_ui::prelude::Header;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A document layout with a `<meta charset>` head; the content fills `<main>`.
	#[template]
	fn PageLayout() -> impl Bundle {
		rsx! {
			<html>
				<head><meta charset="utf-8"/></head>
				<body><main><Slot/></main></body>
			</html>
		}
	}

	/// A layout that places the content inside `<nav>`.
	#[template]
	fn NavLayout() -> impl Bundle {
		rsx! { <body><nav><Slot/></nav></body> }
	}

	/// Request `path`, negotiating HTML, and return the rendered body.
	async fn get(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.exchange(
				Request::get(path)
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap_str()
			.await
	}

	#[beet_core::test]
	async fn wraps_fixed_route() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<PageLayout>::default(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		// layout + page body present, transcluded in place
		html.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<p>page body</p>");
	}

	#[beet_core::test]
	async fn fixed_route_survives_repeat_requests() {
		// the shared fixed content must not be despawned with the layout; each
		// request must render identically (the despawn-hazard regression).
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<PageLayout>::default(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"page body"</p> }
				)
			]))
			.flush();

		let first = get(&mut world, root, "").await;
		let second = get(&mut world, root, "").await;
		second.as_str().xpect_contains("<p>page body</p>");
		first.xpect_eq(second);
	}

	#[beet_core::test]
	async fn wraps_async_route() {
		async fn page(_cx: ActionContext<Request>) -> impl Bundle {
			rsx! { <p>"async body"</p> }
		}
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<PageLayout>::default(), children![
				render_action::async_route("", page)
			]))
			.flush();

		// per-request content is ephemeral; render twice to prove cleanup
		for _ in 0..2 {
			get(&mut world, root, "")
				.await
				.as_str()
				.xpect_contains("<meta charset=\"utf-8\"")
				.xpect_contains("<p>async body</p>");
		}
	}

	#[beet_core::test]
	async fn wraps_blob_scene_markdown() {
		let store = BlobStore::temp();
		store
			.insert(&"post.md".into(), "# Hello\n\nmarkdown body".to_owned())
			.await
			.unwrap();

		let mut world = router_world();
		let root = world
			.spawn((
				store,
				Router,
				BaseLayout::<PageLayout>::default(),
				children![route("post", BlobScene::new("post.md"))],
			))
			.flush();

		// the markdown content (parsed per request) lands inside the layout's
		// `main`, transcluded by reference
		get(&mut world, root, "post")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("markdown body");
	}

	/// Repeated requests to the same markdown route must not leak entities: the
	/// per-request layout subtree and content ephemerals are despawned after
	/// render, and the persistent route tree is diffed in place. A growing entity
	/// count is the ramp-up bug (a page got slower with every visit because the
	/// post-parse pipeline re-scanned ever more resident entities).
	#[beet_core::test]
	async fn repeated_requests_stay_bounded() {
		let store = BlobStore::temp();
		store
			.insert(
				&"post.md".into(),
				"# Title\n\n```rust\nfn main() {}\n```\n\nbody".to_owned(),
			)
			.await
			.unwrap();

		let mut world = router_world();
		let root = world
			.spawn((
				store,
				Router,
				BaseLayout::<PageLayout>::default(),
				children![route("post", BlobScene::new("post.md"))],
			))
			.flush();

		// warm up so the route's tree is parsed and any one-off resources settle,
		// then sample the entity count and confirm it holds flat across requests.
		get(&mut world, root, "post").await;
		let baseline = world.iter_entities().count();
		for _ in 0..8 {
			get(&mut world, root, "post").await;
		}
		world.iter_entities().count().xpect_eq(baseline);
	}

	#[beet_core::test]
	async fn layout_places_content_where_it_chooses() {
		// the layout decides placement; here the content lands inside <nav>
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<NavLayout>::default(), children![
				render_action::fixed_func_route("", || rsx! { <a>"home"</a> })
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		let nav_open = html.find("<nav>").unwrap();
		let nav_close = html.find("</nav>").unwrap();
		let link = html.find("<a>home</a>").unwrap();
		link.xpect_greater_than(nav_open);
		link.xpect_less_than(nav_close);
	}

	/// The real site head/header layout: `RouteHead` owns the single `<title>`
	/// bound to the route, `Header` owns the site-title link.
	#[cfg(feature = "json")]
	#[template]
	fn MetaLayout() -> impl Bundle {
		rsx! {
			<html>
				<RouteHead/>
				<body><Header/><main><Slot/></main></body>
			</html>
		}
	}

	/// A route whose rendered content carries the given frontmatter title.
	#[cfg(feature = "json")]
	fn meta_route(path: &str, title: &str) -> impl Bundle {
		let meta = ArticleMeta {
			title: Some(title.into()),
			..default()
		};
		render_action::fixed_func_route(path, move || {
			(meta.clone(), rsx! { <p>"body"</p> })
		})
	}

	/// The sticky-title regression: through the real `wrap_content_with` pipeline
	/// (per-request layout + transcluded content + fresh `LayoutContent`), each
	/// route renders its *own* `<title>` (not the previous request's), the visible
	/// header stays the site title, and the shared `PackageConfig.title` is never
	/// polluted by a per-route title write-back.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn route_title_differs_per_request_and_header_stays_site_title() {
		let mut world = router_world();
		world.insert_resource(PackageConfig {
			title: "SiteName".into(),
			..default()
		});
		let root = world
			.spawn((Router, BaseLayout::<MetaLayout>::default(), children![
				meta_route("alpha", "Alpha"),
				meta_route("beta", "Beta"),
			]))
			.flush();

		// each route renders exactly one `<title>`, carrying its own route title.
		let alpha = get(&mut world, root, "alpha").await;
		alpha.matches("<title>").count().xpect_eq(1);
		alpha.as_str().xpect_contains("<title>Alpha</title>");
		// the header link is always the site title, never the route title.
		alpha.as_str().xpect_contains("app-bar-title");
		alpha
			.split("app-bar-title")
			.nth(1)
			.unwrap()
			.xpect_contains("SiteName");

		// a different route renders a different title (not sticky on "Alpha").
		let beta = get(&mut world, root, "beta").await;
		beta.as_str().xpect_contains("<title>Beta</title>");
		beta.as_str().xnot().xpect_contains("<title>Alpha</title>");

		// re-requesting alpha is fresh again, not stuck on the last request.
		get(&mut world, root, "alpha")
			.await
			.as_str()
			.xpect_contains("<title>Alpha</title>");

		// the shared resource was never overwritten by a per-route title.
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("SiteName");
	}

	/// The per-request link is fresh: `wrap_content_with` installs a
	/// [`LayoutContent`] from the layout root to *this request's* content, the
	/// seam the layout-head title binding follows. Two requests yield two distinct
	/// content entities, each pointed at by its own layout.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn wrap_content_links_each_request_to_fresh_content() {
		let mut world = router_world();
		let content_a = world.spawn(ArticleMeta::default()).flush();
		PageRoot::insert(&mut world.entity_mut(content_a), default());
		let content_b = world.spawn(ArticleMeta::default()).flush();
		PageRoot::insert(&mut world.entity_mut(content_b), default());

		// these test contents are self-referential roots, so the route anchor and
		// content coincide.
		let layout_a = wrap_content_with(
			&mut world,
			RequestParts::get("alpha"),
			content_a,
			content_a,
			|world, _| Ok(world.spawn(Element::new("html")).id()),
		)
		.unwrap();
		let layout_b = wrap_content_with(
			&mut world,
			RequestParts::get("beta"),
			content_b,
			content_b,
			|world, _| Ok(world.spawn(Element::new("html")).id()),
		)
		.unwrap();

		// each layout points at its own request's content, not a shared/stale one.
		world
			.entity(layout_a)
			.get::<LayoutContent>()
			.unwrap()
			.0
			.xpect_eq(content_a);
		world
			.entity(layout_b)
			.get::<LayoutContent>()
			.unwrap()
			.0
			.xpect_eq(content_b);
		layout_a.xpect_not_eq(layout_b);
	}
}
