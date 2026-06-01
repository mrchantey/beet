//! Layout render middleware: wrap a route's rendered content in a document
//! shell (the web `<html>`/`<head>` document, an article/sidebar layout, etc.)
//! without reparenting or re-resolving it.
//!
//! [`document_shell`] registers a render middleware on its entity. For every
//! descendant render route, the middleware runs the inner handler to obtain the
//! content render root, spawns the shell scene, and points the shell's content
//! `<slot>` at the existing content entity via [`SlotContainer`]. The content is
//! rendered *in place, by reference*: it is never reparented under the shell nor
//! re-resolved, so a persistent fixed route survives request after request.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
// disambiguate the slot-wiring `spawn_scene` from bevy's `WorldSceneExt`, which
// is also in scope via `beet_core::prelude`.
use beet_ui::prelude::WorldSceneExt;

/// Wrap every descendant render route in a document shell `scene`, slotting the
/// route's content into the shell's `<slot name="main">`.
///
/// The shell is a `#[scene]`/`rsx!` constructor returning the outer document
/// (eg `<html><head>…</head><body><slot name="main"/></body></html>`); it is
/// invoked fresh per request. Place it on an ancestor of the routes it should
/// wrap, eg the router entity:
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// # fn shell() -> impl Scene { rsx! { <html><body><slot name="main"/></body></html> } }
/// # fn routes() -> impl Bundle {}
/// let bundle = (router(), document_shell(shell), children![routes()]);
/// ```
pub fn document_shell<Func, S>(scene: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn() -> S,
	S: 'static + Send + Sync + Scene,
{
	layout_middleware("main", scene)
}

/// The generalized form of [`document_shell`]: slot route content into the
/// shell's `<slot name="{content_slot}">` instead of the default `"main"`.
pub fn layout_middleware<Func, S>(
	content_slot: impl Into<String>,
	scene: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn() -> S,
	S: 'static + Send + Sync + Scene,
{
	let content_slot = content_slot.into();
	let action: Action<(RequestParts, Next<RequestParts, Entity>), Entity> =
		(move |parts: RequestParts, next: Next<RequestParts, Entity>| {
			let scene = scene.clone();
			let content_slot = content_slot.clone();
			async move {
				// resolve the inner content render root, then wrap it
				let content = next.call(parts).await?;
				next.world()
					.clone()
					.with(move |world: &mut World| {
						wrap_content(world, content, scene, &content_slot)
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

/// Spawn the shell `scene` and slot the existing `content` render root into its
/// `content_slot`, returning the shell as the new render root.
///
/// The shell is ephemeral: it (and its subtree) are despawned after render,
/// along with the inner content's own ephemerals. The slotted content is
/// referenced via [`SlotContainer`], not owned, so it is never despawned here.
fn wrap_content<S: Scene>(
	world: &mut World,
	content: Entity,
	scene: impl FnOnce() -> S,
	content_slot: &str,
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

	// spawn the shell and point its content slot at the existing content
	let layout = world.spawn_scene(scene())?.id();
	let slot = find_named_slot(world, layout, content_slot).ok_or_else(|| {
		bevyhow!("layout shell has no `<slot name=\"{content_slot}\">`")
	})?;
	world.entity_mut(slot).insert(SlotContainer::new(rendered));

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

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A document shell with a `<meta charset>` head and a `main` content slot.
	fn shell() -> impl Scene {
		rsx! {
			<html>
				<head><meta charset="utf-8"/></head>
				<body><main><slot name="main"/></main></body>
			</html>
		}
	}

	/// A shell whose only slot is a `nav` slot, for named-slot routing.
	fn nav_shell() -> impl Scene {
		rsx! { <body><nav><slot name="nav"/></nav></body> }
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
			.spawn((
				router(),
				document_shell(shell),
				children![render_action::fixed_route(
					"",
					rsx_direct! { <p>"page body"</p> }
				)],
			))
			.flush();

		let html = get(&mut world, root, "").await;
		// shell + page body present, the slot element itself is transparent
		html.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<p>page body</p>")
			.xnot()
			.xpect_contains("<slot");
	}

	#[beet_core::test]
	async fn fixed_route_survives_repeat_requests() {
		// the shared fixed content must not be despawned with the shell; each
		// request must render identically (the despawn-hazard regression).
		let mut world = router_world();
		let root = world
			.spawn((
				router(),
				document_shell(shell),
				children![render_action::fixed_route(
					"",
					rsx_direct! { <p>"page body"</p> }
				)],
			))
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
			.spawn((
				router(),
				document_shell(shell),
				children![render_action::async_route("", page)],
			))
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
				router(),
				document_shell(shell),
				children![route("post", BlobScene::new("post.md"))],
			))
			.flush();

		// the markdown content (parsed per request) lands inside the shell's
		// `main`; its `<slot>` is consumed
		get(&mut world, root, "post")
			.await
			.as_str()
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("markdown body")
			.xnot()
			.xpect_contains("<slot");
	}

	#[beet_core::test]
	async fn routes_into_named_slot() {
		// content slotted into `nav` instead of `main`; it lands inside <nav>
		let mut world = router_world();
		let root = world
			.spawn((
				router(),
				layout_middleware("nav", nav_shell),
				children![render_action::fixed_route(
					"",
					rsx_direct! { <a>"home"</a> }
				)],
			))
			.flush();

		let html = get(&mut world, root, "").await;
		let nav_open = html.find("<nav>").unwrap();
		let nav_close = html.find("</nav>").unwrap();
		let link = html.find("<a>home</a>").unwrap();
		link.xpect_greater_than(nav_open);
		link.xpect_less_than(nav_close);
	}
}
