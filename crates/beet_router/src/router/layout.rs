//! Layout render middleware: wrap a route's rendered content in a document
//! shell (the web `<html>`/`<head>` document, an article/sidebar layout, etc.)
//! without reparenting or re-resolving it.
//!
//! [`document_shell`] registers a render middleware on its entity. For every
//! descendant render route, the middleware runs the inner handler to obtain the
//! content render root, then builds the shell — an ordinary `#[scene]` widget —
//! with the content as its `children` prop (a [`RenderRef`] transclusion). The
//! content is rendered *in place, by reference*: it is never reparented under
//! the shell nor re-resolved, so a persistent fixed route survives request after
//! request.
//!
//! The shell wraps **every** request regardless of target. Non-visual document
//! chrome (`<head>`/`<style>`/`<script>`) simply does not paint in the terminal
//! (it resolves to `display: none`; see the user-agent style layer), so the same
//! shell renders correctly on web and terminal.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Wrap every descendant render route in a document shell `C`.
///
/// The shell is just a `#[scene]` widget with a default `<slot/>`: the route's
/// rendered content is built into its `children` prop and the widget places it
/// (directly, or by forwarding to a [`PageLayout`]). It is invoked fresh per
/// request. Place it on an ancestor of the routes it should wrap, eg the router
/// entity:
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// #[scene]
/// fn Shell() -> impl Scene { rsx! { <html><body><slot/></body></html> } }
/// let bundle = (router(), document_shell::<Shell>());
/// ```
pub fn document_shell<C>() -> impl Bundle
where
	C: 'static + Send + Sync + WithChildren,
{
	shell_middleware(|content: SceneProp| C::scene(C::with_children(content)))
}

/// The render-middleware machinery behind [`document_shell`]: register an action
/// that wraps every descendant render route's content via the `shell` builder.
fn shell_middleware<Func, S>(shell: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(SceneProp) -> S,
	S: Scene,
{
	let action: Action<(RequestParts, Next<RequestParts, Entity>), Entity> =
		(move |parts: RequestParts, next: Next<RequestParts, Entity>| {
			let shell = shell.clone();
			async move {
				// resolve the inner content render root, then wrap it
				let content = next.call(parts.clone()).await?;
				// the request path feeds the render context (active nav, etc.)
				let path = parts.path().join("/");
				next.world()
					.clone()
					.with(move |world: &mut World| {
						wrap_content(world, &path, content, shell)
					})
					.await
			}
		})
		.into_action();

	OnSpawn::new(move |entity: &mut EntityWorldMut| {
		entity
			.get_mut_or_default::<MiddlewareList<RequestParts, Entity>>()
			.0
			.push(action);
	})
}

/// Spawn the `shell` around the existing `content` render root, returning the
/// shell as the new render root.
///
/// The content is handed to the shell as a [`SceneProp`] wrapping a
/// [`RenderRef`]: the shell places it via its `<slot/>`, transcluding the
/// existing content entity **by reference**. The shell subtree is ephemeral and
/// despawned after render (along with the content's own ephemerals), but the
/// referenced content is never owned or despawned here, so a persistent fixed
/// route survives request after request.
fn wrap_content<Func, S>(
	world: &mut World,
	path: &str,
	content: Entity,
	shell: Func,
) -> Result<Entity>
where
	Func: FnOnce(SceneProp) -> S,
	S: Scene,
{
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

	// the request-scoped route context, read by the shell's scene systems: the
	// current path plus the matched route's metadata (parsed from frontmatter
	// onto the content entity). Installed as a resource for the synchronous
	// shell build, then removed.
	let article_meta =
		world.entity(rendered).get::<ArticleMeta>().cloned().unwrap_or_default();
	world.insert_resource(RouteContext::new(path, article_meta));

	// the content is transcluded by reference: the shell places this prop, which
	// resolves to a transparent entity pointing at the existing content.
	let content_prop = SceneProp::new(template_value(RenderRef::new(rendered)));
	let layout = world.spawn_scene(shell(content_prop))?.id();
	world.remove_resource::<RouteContext>();

	// despawn the shell subtree plus the content's ephemerals after render
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
	use beet_ui::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A document shell with a `<meta charset>` head; the content fills `<main>`.
	#[scene]
	fn Shell() -> impl Scene {
		rsx! {
			<html>
				<head><meta charset="utf-8"/></head>
				<body><main><slot/></main></body>
			</html>
		}
	}

	/// A shell that places the content inside `<nav>`.
	#[scene]
	fn NavShell() -> impl Scene {
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
			.spawn((router(), document_shell::<Shell>(), children![
				render_action::fixed_route(
					"",
					rsx_direct! { <p>"page body"</p> }
				)
			]))
			.flush();

		let html = get(&mut world, root, "").await;
		// shell + page body present, transcluded in place
		html.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<p>page body</p>");
	}

	#[beet_core::test]
	async fn fixed_route_survives_repeat_requests() {
		// the shared fixed content must not be despawned with the shell; each
		// request must render identically (the despawn-hazard regression).
		let mut world = router_world();
		let root = world
			.spawn((router(), document_shell::<Shell>(), children![
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
			.spawn((router(), document_shell::<Shell>(), children![
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
			.spawn((store, router(), document_shell::<Shell>(), children![route(
				"post",
				BlobScene::new("post.md")
			)]))
			.flush();

		// the markdown content (parsed per request) lands inside the shell's
		// `main`, transcluded by reference
		get(&mut world, root, "post")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("markdown body");
	}

	#[beet_core::test]
	async fn shell_places_content_where_it_chooses() {
		// the shell decides placement; here the content lands inside <nav>
		let mut world = router_world();
		let root = world
			.spawn((router(), document_shell::<NavShell>(), children![
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
