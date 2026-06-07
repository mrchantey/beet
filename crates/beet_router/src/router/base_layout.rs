//! Layout render middleware: wrap a route's rendered content in a document
//! layout (the web `<html>`/`<head>` document, an article/sidebar layout, etc.)
//! without reparenting or re-resolving it.
//!
//! [`BaseLayout`] is a render-middleware component (registered like any other
//! middleware, eg [`RequestLogger`]). For every descendant render route it runs
//! the inner handler to obtain the content render root, then builds the layout,
//! an ordinary `#[scene]` widget, with the content as its `children` prop (a
//! [`RenderRef`] transclusion). The content is rendered *in place, by reference*:
//! it is never reparented under the layout nor re-resolved, so a persistent fixed
//! route survives request after request.
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
/// layout widget `C` — an ordinary `#[scene]` widget with a default `<slot/>`.
///
/// Add it to an ancestor of the routes it should wrap (eg the router entity),
/// exactly like any other middleware ([`RequestLogger`], [`HelpHandler`]):
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// #[scene]
/// fn PageLayout() -> impl Scene { rsx! { <html><body><slot/></body></html> } }
/// let bundle = (Router, BaseLayout::<PageLayout>::default());
/// ```
///
/// For each request it runs the inner route to obtain the content render root,
/// then builds `C` with that content as its `children` prop (a [`RenderRef`]
/// transclusion placed by the layout's `<slot/>`).
#[action]
#[derive(Component)]
#[component(on_add = on_add_middleware::<Self, RequestParts, Entity>)]
pub async fn BaseLayout<C>(
	cx: ActionContext<(RequestParts, Next<RequestParts, Entity>)>,
) -> Result<Entity>
where
	C: 'static + Send + Sync + Clone + WithChildren,
{
	let (parts, next) = &cx.input;
	// resolve the inner content render root, then wrap it
	let content = next.call(parts.clone()).await?;
	// the request parts feed the render context (active nav, per-route meta, etc.)
	let parts = parts.clone();
	next.world()
		.clone()
		.with(move |world: &mut World| wrap_content::<C>(world, parts, content))
		.await
}

/// Spawn the layout `C` around the existing `content` render root, returning the
/// layout as the new render root.
///
/// The content is handed to the layout as a [`SceneProp`] wrapping a
/// [`RenderRef`]: the layout places it via its `<slot/>`, transcluding the
/// existing content entity **by reference**. The layout subtree is ephemeral and
/// despawned after render (along with the content's own ephemerals), but the
/// referenced content is never owned or despawned here, so a persistent fixed
/// route survives request after request.
fn wrap_content<C: WithChildren>(
	world: &mut World,
	parts: RequestParts,
	content: Entity,
) -> Result<Entity> {
	// the inner render root names the entity to render and its ephemerals
	let (rendered, content_despawn) = {
		let entity = world.entity(content);
		let rendered = entity
			.get::<RenderRoot>()
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

	// the request-scoped render context, read by the layout's scene systems: the
	// request parts plus the rendered content entity, off which widgets query
	// any per-route components (eg `ArticleMeta` parsed from frontmatter).
	// Installed as a resource for the synchronous layout build, then removed.
	world.insert_resource(RequestContext::new(parts, rendered));

	// the content is transcluded by reference: the layout places this prop, which
	// resolves to a transparent entity pointing at the existing content.
	let content_prop = SceneProp::new(template_value(RenderRef::new(rendered)));
	let layout = world
		.spawn_scene(C::scene(C::with_children(content_prop)))?
		.id();
	world.remove_resource::<RequestContext>();

	// despawn the layout subtree plus the content's ephemerals after render
	let mut to_despawn = vec![layout];
	to_despawn.extend(content_despawn);
	RenderRoot::insert(&mut world.entity_mut(layout), to_despawn);
	layout.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A document layout with a `<meta charset>` head; the content fills `<main>`.
	#[scene]
	fn PageLayout() -> impl Scene {
		rsx! {
			<html>
				<head><meta charset="utf-8"/></head>
				<body><main><slot/></main></body>
			</html>
		}
	}

	/// A layout that places the content inside `<nav>`.
	#[scene]
	fn NavLayout() -> impl Scene {
		rsx! { <body><nav><slot/></nav></body> }
	}

	/// Request `path`, negotiating HTML, and return the rendered body.
	async fn get(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::get(path)
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
	}

	#[beet_core::test]
	async fn wraps_fixed_route() {
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<PageLayout>::default(), children![
				render_action::fixed_route(
					"",
					rsx_direct! { <p>"page body"</p> }
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
				render_action::fixed_route(
					"",
					rsx_direct! { <p>"page body"</p> }
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
			rsx_direct! { <p>"async body"</p> }
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

	#[beet_core::test]
	async fn layout_places_content_where_it_chooses() {
		// the layout decides placement; here the content lands inside <nav>
		let mut world = router_world();
		let root = world
			.spawn((Router, BaseLayout::<NavLayout>::default(), children![
				render_action::fixed_route("", rsx_direct! { <a>"home"</a> })
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		let nav_open = html.find("<nav>").unwrap();
		let nav_close = html.find("</nav>").unwrap();
		let link = html.find("<a>home</a>").unwrap();
		link.xpect_greater_than(nav_open);
		link.xpect_less_than(nav_close);
	}
}
