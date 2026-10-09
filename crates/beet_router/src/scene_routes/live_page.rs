//! A page built and held alive between its build and its renders.
//!
//! [`LivePage::prepare`] runs a built page's render middleware (the layouts),
//! resolves the `--root` cascade ([`RenderRoot`]) and, when the request names
//! a [`MediaIngestPolicy`], the media resolve step ([`InlineBlob`]). The live
//! page renders through any registered target as many times as its holder
//! likes, and its release takes back everything the preparation put on a tree
//! it does not own. [`LivePage::scoped`] and [`LivePage::respond`] release it
//! whatever the outcome; a live surface binds one instead.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A page built and held alive: its render middleware run (the layouts), the
/// `--root` cascade resolved, the route's [`PageMeta`] and its [`PageUrl`]
/// carried onto the root it chose, and the media its request named resolved.
/// Rendered through any registered target as many times as its holder likes.
///
/// [`LivePage::scoped`] and [`LivePage::respond`] release it whatever the
/// outcome, so an early `?` never strands a page; a live surface binds one
/// instead ([`into_surface`](Self::into_surface)).
#[derive(Clone)]
pub struct LivePage {
	world: AsyncWorld,
	/// The whole page, its layouts included, on which its ephemerals are
	/// recorded.
	page: Entity,
	/// The entity a render starts from, the cascade's answer to `--root`.
	root: Entity,
	/// The ephemeral entities released with the page.
	to_despawn: Vec<Entity>,
	/// The root a copy of the route's [`PageMeta`] was inserted on, taken back
	/// on release.
	carried_meta: Option<Entity>,
	/// The entities the media resolve step gave an [`InlineBlob`], taken back
	/// on release so a persistent tree keeps no fetched bytes.
	inline_blobs: Vec<Entity>,
}

/// The url the page a render root belongs to answers at: absolute on the
/// site's homepage when the site declares one, else rooted.
///
/// Carried onto the render root by [`LivePage::prepare`] for the life of the
/// live page, so a target writing links a reader follows elsewhere (a Leaflet
/// document) resolves the page's relative links as a browser would.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct PageUrl(pub Url);

/// A page bound to a live surface: the `page` whose ephemerals the surface
/// reclaims on the next swap, and the entity `shown` in the surface's slot,
/// the page itself unless a `--root` chose part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfacePage {
	pub page: Entity,
	pub shown: Entity,
}

impl From<Entity> for SurfacePage {
	fn from(page: Entity) -> Self { Self { page, shown: page } }
}

impl LivePage {
	/// Prepare a built `page` for rendering: run the render middleware of
	/// `route` and its ancestors (the layouts), resolve the `--root` cascade
	/// ([`RenderRoot`]), carry the route's [`PageMeta`] onto the root it chose,
	/// so a scene response reads the page's metadata where it reads the tree,
	/// and, when the request names a [`MediaIngestPolicy`], run the media
	/// resolve step ([`InlineBlob::resolve`]). A failure releases what was
	/// built.
	pub async fn prepare(
		page: Entity,
		route: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Self> {
		let params = match RenderParams::of(&parts) {
			Ok(params) => params,
			Err(err) => {
				Self::discard(route.world(), page).await;
				return Err(err);
			}
		};
		let page_url = Url::coerce(parts.path_string());
		// apply ancestor render middleware (layout wrapping, etc.)
		let wrapped = match route
			.call_with_middleware(Action::new_fixed(page), parts)
			.await
		{
			Ok(wrapped) => wrapped,
			Err(err) => {
				Self::discard(route.world(), page).await;
				return Err(err);
			}
		};
		let world = route.world().clone();
		let route_id = route.id();
		let page_url_carried = page_url.clone();
		let prepared = route
			.world()
			.with(move |world_mut: &mut World| -> Result<(Self, Entity)> {
				let entity = world_mut.entity(wrapped);
				let rendered = entity
					.get::<PageRoot>()
					.ok_or_else(|| {
						bevyhow!("entity {wrapped} is not a render root")
					})?
					.rendered();
				let to_despawn = entity
					.get::<DespawnAfterRender>()
					.map(|despawn| despawn.0.clone())
					.unwrap_or_default();
				let resolved =
					world_mut.with_state::<RenderRootQuery, _>(|query| {
						query.resolve(rendered, params.root)
					});
				// the route's metadata lives on its content, the end of any
				// layout chain, which the resolved root generally is not
				let content = LayoutContent::terminal(world_mut, rendered)?;
				let carried_meta = world_mut
					.get::<PageMeta>(content)
					.filter(|_| {
						!world_mut.entity(resolved).contains::<PageMeta>()
					})
					.cloned()
					.map(|meta| {
						world_mut.entity_mut(resolved).insert(meta);
						resolved
					});
				let absolute = world_mut
					.get_resource::<PackageConfig>()
					.and_then(|package| package.homepage.as_ref())
					.map(|homepage| homepage.join(page_url_carried.clone()))
					.unwrap_or(page_url_carried);
				world_mut.entity_mut(resolved).insert(PageUrl(absolute));
				// the router a source fetch re-enters
				let router = world_mut
					.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
						trees.get_entity(route_id)
					})
					.unwrap_or(route_id);
				let live = Self {
					world,
					page: wrapped,
					root: resolved,
					to_despawn,
					carried_meta,
					inline_blobs: Vec::new(),
				};
				(live, router).xok()
			})
			.await;
		let (mut live, router) = match prepared {
			Ok(prepared) => prepared,
			Err(err) => {
				Self::discard(route.world(), wrapped).await;
				return Err(err);
			}
		};
		// a request naming no policy fetches nothing
		let Some(policy) = params.media_ingest else {
			return live.xok();
		};
		match InlineBlob::resolve(
			&live.world,
			router,
			page_url,
			live.root,
			policy,
		)
		.await
		{
			Ok(inline_blobs) => {
				live.inline_blobs = inline_blobs;
				live.xok()
			}
			Err(err) => {
				live.release().await;
				Err(err)
			}
		}
	}

	/// Match `request` against the route tree `router` sits in, build the
	/// matched scene route and [`prepare`](Self::prepare) it. The caller owns
	/// the live page: [`scoped`](Self::scoped) releases it, a live surface
	/// binds it.
	pub async fn prepare_request(
		router: &AsyncEntity,
		request: Request,
	) -> Result<Self> {
		let (route, page, parts) = Self::build(router, request).await?;
		Self::prepare(page, &route, parts).await
	}

	/// Prepare the page `request` names, hand it to `func` to render through
	/// any target, then release it whether `func` succeeds or fails, so no
	/// page outlives its scope.
	///
	/// A request is this scope around one render whose target negotiation
	/// picks from `Accept`, so a direct caller naming the same media type gets
	/// exactly the body a client does.
	pub async fn scoped<T, Fut>(
		router: &AsyncEntity,
		request: Request,
		func: impl FnOnce(LivePage) -> Fut,
	) -> Result<T>
	where
		Fut: Future<Output = Result<T>>,
	{
		Self::prepare_request(router, request)
			.await?
			.release_after(func)
			.await
	}

	/// The response a request makes of a built `page`: negotiate the media
	/// type from `Accept`, prepare the page, render it and release it.
	///
	/// This is the http page handler, so html is the preferred type: a request
	/// with no `Accept` or a wildcard (`*/*`) renders the web document.
	pub async fn respond(
		page: Entity,
		route: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Response> {
		let accepts: Vec<MediaType> = parts
			.headers
			.get::<header::Accept>()
			.and_then(|result| result.ok())
			.unwrap_or_default();
		let negotiated = route
			.world()
			.with(move |world: &mut World| -> Result<MediaType> {
				world
					.get_resource::<RenderTargets>()
					.ok_or_else(|| {
						bevyhow!(
							"no `RenderTargets` in this world: add the `RenderPlugin`"
						)
					})?
					.negotiate(&accepts, &MediaType::Html)?
					.xok()
			})
			.await;
		let media_type = match negotiated {
			Ok(media_type) => media_type,
			Err(err) => {
				Self::discard(route.world(), page).await;
				return Err(err);
			}
		};
		let bytes = Self::prepare(page, route, parts)
			.await?
			.release_after(async |live| live.render(&media_type).await)
			.await?;
		Response::ok().with_media(bytes).xok()
	}

	/// The entity a render starts from, the cascade's answer to `--root`.
	pub fn root(&self) -> Entity { self.root }

	/// Every [`InlineBlob`] the media resolve step fetched onto this page,
	/// the cover image's included.
	pub async fn inline_blobs(&self) -> Vec<InlineBlob> {
		let entities = self.inline_blobs.clone();
		self.world
			.with(move |world: &mut World| {
				entities
					.iter()
					.filter_map(|entity| {
						world.get::<InlineBlob>(*entity).cloned()
					})
					.collect()
			})
			.await
	}

	/// The page's cover image, the [`InlineBlob`] the media resolve step
	/// fetched onto the render root from `PageMeta::social_image_url`.
	pub async fn cover(&self) -> Option<InlineBlob> {
		let root = self.root;
		self.world
			.with(move |world: &mut World| {
				world.get::<InlineBlob>(root).cloned()
			})
			.await
	}

	/// Render the page as `media_type`, through the target registered for it.
	pub async fn render(&self, media_type: &MediaType) -> Result<MediaBytes> {
		let root = self.root;
		let media_type = media_type.clone();
		self.world
			.with(move |world: &mut World| {
				RenderTargets::render(world, root, &media_type)
			})
			.await?
			.xok()
	}

	/// Hand the page to a live surface rather than releasing it: the surface
	/// shows the cascade's root and reclaims the page's ephemerals when the
	/// next page replaces it.
	pub(crate) fn into_surface(self) -> SurfacePage {
		SurfacePage {
			page: self.page,
			shown: self.root,
		}
	}

	/// Match `request` against the route tree `router` sits in and build the
	/// matched scene route, answering the route, the page it built and the
	/// request's parts with the matched path params merged in.
	async fn build(
		router: &AsyncEntity,
		mut request: Request,
	) -> Result<(AsyncEntity, Entity, RequestParts)> {
		let path = request.path().clone();
		let router_id = router.id();
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
			.await?
			.ok_or_else(|| {
				bevyhow!("no route matched {}", request.path_string())
			})?;
		// surface matched dynamic segments (`:id`) to the handler
		node.merge_path_params(&mut request);
		let parts = request.parts().clone();
		let route = router.world().entity(node.entity);
		// the route's own content through its canonical action, skipping the
		// `ExchangeOverload` adapter that would render and release it
		let page = route.call::<Request, PageRequest>(request).await?.0;
		(route, page, parts).xok()
	}

	/// Run `func` over this page, then release it whatever `func` answered.
	async fn release_after<T, Fut>(
		self,
		func: impl FnOnce(LivePage) -> Fut,
	) -> Result<T>
	where
		Fut: Future<Output = Result<T>>,
	{
		let result = func(self.clone()).await;
		self.release().await;
		result
	}

	/// Despawn the page's ephemerals and take back what was carried onto a
	/// tree it does not own: the metadata, the url and the fetched media.
	async fn release(self) {
		self.world
			.with(move |world: &mut World| {
				if let Some(entity) = self.carried_meta
					&& let Ok(mut entity) = world.get_entity_mut(entity)
				{
					entity.remove::<PageMeta>();
				}
				if let Ok(mut root) = world.get_entity_mut(self.root) {
					root.remove::<PageUrl>();
				}
				for entity in self.inline_blobs {
					if let Ok(mut entity) = world.get_entity_mut(entity) {
						entity.remove::<InlineBlob>();
					}
				}
				Self::despawn(world, self.to_despawn);
			})
			.await;
	}

	/// Release a built page that never became live, ie its render middleware
	/// or negotiation failed.
	async fn discard(world: &AsyncWorld, page: Entity) {
		world
			.with(move |world: &mut World| {
				let to_despawn = world
					.get_entity(page)
					.ok()
					.and_then(|entity| entity.get::<DespawnAfterRender>())
					.map(|despawn| despawn.0.clone())
					.unwrap_or_default();
				Self::despawn(world, to_despawn);
			})
			.await;
	}

	fn despawn(world: &mut World, entities: Vec<Entity>) {
		for entity in entities {
			if let Ok(entity) = world.get_entity_mut(entity) {
				entity.despawn();
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The path of the fixture's article, which wears both layouts.
	const POST: &str = "/blog/full-stack-bevy";

	/// The layout fixture site's world and router.
	fn site() -> (World, Entity) {
		let mut world = syndication_world(Some("https://beet.org"));
		let router = spawn_layout_router(&mut world);
		(world, router)
	}

	/// `path` under `root`, ie `?root=main`.
	fn at(path: &str, root: &str) -> Request {
		Request::get(path).with_param("root", root)
	}

	/// Each root a render can take, `None` the whole page.
	const ROOTS: [Option<&str>; 3] = [None, Some("main"), Some("content")];

	/// The fixture's article at `root`, the whole page for none.
	fn post_at(root: Option<&str>) -> Request {
		match root {
			Some(root) => at(POST, root),
			None => Request::get(POST),
		}
	}

	/// The body `request` answers as `media_type`.
	async fn get(
		world: &mut World,
		router: Entity,
		request: Request,
		media_type: MediaType,
	) -> String {
		world
			.entity_mut(router)
			.exchange(request.with_accept(media_type))
			.await
			.unwrap_str()
			.await
	}

	/// The body `request` renders as `media_type` through the scope, as a
	/// direct caller holding the live tree renders it.
	async fn scoped(
		world: &mut World,
		router: Entity,
		request: Request,
		media_type: MediaType,
	) -> String {
		world
			.run_async_then(async move |world| {
				LivePage::scoped(&world.entity(router), request, async |live| {
					live.render(&media_type).await
				})
				.await
			})
			.await
			.unwrap()
			.to_string()
	}

	/// How many entities the world holds, so a released page shows as none
	/// left behind.
	fn entities(world: &mut World) -> usize {
		world.query::<Entity>().iter(world).count()
	}

	/// The whole page by default, `main` the layout's main content with the
	/// article header, `content` the route's own content alone: html answers
	/// a fragment with no head, no description and no stylesheet.
	#[beet_core::test]
	async fn html_at_each_root() {
		let (mut world, router) = site();
		get(&mut world, router, Request::get(POST), MediaType::Html)
			.await
			.xpect_contains("<html>")
			.xpect_contains("<nav>Home</nav>")
			.xpect_contains("<h1>Full Stack Bevy</h1>")
			.xpect_contains("<p>page</p>");
		get(&mut world, router, at(POST, "main"), MediaType::Html)
			.await
			.xpect_starts_with("<main>")
			.xpect_contains("<h1>Full Stack Bevy</h1>")
			.xpect_contains("<p>page</p>")
			.xnot()
			.xpect_contains("<head>")
			.xnot()
			.xpect_contains("<style>")
			.xnot()
			.xpect_contains("<nav>");
		get(&mut world, router, at(POST, "content"), MediaType::Html)
			.await
			.xpect_eq("<p>page</p>");
	}

	/// Markdown and plain text answer the main content alone, and `content`
	/// leaves out what the article layout contributed.
	#[beet_core::test]
	async fn text_formats_at_each_root() {
		let (mut world, router) = site();
		get(&mut world, router, at(POST, "main"), MediaType::Markdown)
			.await
			.xpect_contains("# Full Stack Bevy")
			.xpect_contains("page")
			.xnot()
			.xpect_contains("Home")
			.xnot()
			.xpect_contains("Bye");
		get(&mut world, router, at(POST, "content"), MediaType::Markdown)
			.await
			.trim()
			.xpect_eq("page");
		get(&mut world, router, at(POST, "main"), MediaType::Text)
			.await
			.xpect_starts_with("Full Stack Bevy\n")
			.xpect_ends_with("page\n")
			.xnot()
			.xpect_contains("Home");
		get(&mut world, router, at(POST, "content"), MediaType::Text)
			.await
			.xpect_eq("page\n");
		get(&mut world, router, at(POST, "content"), MediaType::AnsiTerm)
			.await
			.xpect_contains("page")
			.xnot()
			.xpect_contains("Full Stack Bevy");
	}

	/// A scene response is rooted at the cascade's root, the route's
	/// `PageMeta` carried onto that root at every rung.
	#[cfg(all(feature = "template_serde", feature = "json"))]
	#[beet_core::test]
	async fn scene_root_carries_the_page_meta() {
		let (mut world, router) = site();
		for root in ROOTS {
			get(&mut world, router, post_at(root), MediaType::Json)
				.await
				.xpect_contains("PageMeta")
				.xpect_contains("Full Stack Bevy");
		}
		get(&mut world, router, at(POST, "content"), MediaType::Json)
			.await
			.xnot()
			.xpect_contains("\"h1\"");
	}

	/// The carried `PageMeta` sits on the very root a render starts from,
	/// whichever rung chose it, and goes when the page is released.
	#[beet_core::test]
	async fn carries_the_page_meta_to_each_rung() {
		let mut world = syndication_world(None);
		let meta = post("Rungs", "2025-07-11");
		let route = |path: &str, body: fn() -> Snippet| {
			let meta = meta.clone();
			render_action::fixed_func_route(path, move || {
				(meta.clone(), body())
			})
		};
		let router = world
			.spawn((Router, children![
				route("body", || {
					rsx! { <html><body><p>"page"</p></body></html> }
				}),
				route("mains", || {
					rsx! { <div><div><main>"deep"</main></div><main>"shallow"</main></div> }
				}),
				page("plain", meta.clone()),
			]))
			.flush();
		for (path, tag) in [
			("/body", Some("body")),
			("/mains", Some("main")),
			("/plain", None),
		] {
			let (root_tag, root_meta) = world
				.run_async_then(async move |world| {
					LivePage::scoped(
						&world.entity(router),
						at(path, "main"),
						async |live| {
							let root = live.root();
							world
								.with(move |world: &mut World| {
									(
										world.get::<Element>(root).map(
											|element| element.tag().to_string(),
										),
										world.get::<PageMeta>(root).cloned(),
									)
								})
								.await
								.xok()
						},
					)
					.await
				})
				.await
				.unwrap();
			root_tag.as_deref().xpect_eq(tag.or(Some("p")));
			root_meta.unwrap().title.xpect_eq(Some("Rungs".into()));
		}
		// two `<main>`s: the shallowest wins
		get(&mut world, router, at("/mains", "main"), MediaType::Text)
			.await
			.xpect_eq("shallow\n");
	}

	/// Every built-in target renders each rung of the `main` cascade and the
	/// `content` root: a layout's `<main>` (rung 1, the shallowest of two), a
	/// layout's transcluded content where it has no `<main>` (rung 2), a
	/// page's `<body>` (rung 3) and a page with neither (rung 4), each holding
	/// its own text and none of the chrome around it.
	#[beet_core::test]
	async fn every_target_at_every_rung() {
		let mut world = syndication_world(Some("https://beet.org"));
		world
			.get_resource_or_init::<BsxTemplateRegistry>()
			.insert_source("BareLayout", "<header>Chrome</header><Slot/>")
			.unwrap();
		let meta = post("Rungs", "2025-07-11");
		let route = |path: &str, body: fn() -> Snippet| {
			let meta = meta.clone();
			render_action::fixed_func_route(path, move || {
				(meta.clone(), body())
			})
		};
		let router = world
			.spawn((Router, children![
				(PathPartial::new("bare"), Layout::new("BareLayout"), children![
					route("post", || rsx! { <p>"page"</p> })
				]),
				route("mains", || {
					rsx! { <div><header>"Chrome"</header><div><main>"deep"</main></div><main>"page"</main></div> }
				}),
				route("body", || {
					rsx! { <html><head><title>"Chrome"</title></head><body><p>"page"</p></body></html> }
				}),
				route("plain", || rsx! { <p>"page"</p> }),
			]))
			.flush();
		let targets = [
			MediaType::Html,
			MediaType::Markdown,
			MediaType::Text,
			MediaType::AnsiTerm,
			#[cfg(all(feature = "template_serde", feature = "json"))]
			MediaType::Json,
		];
		// a page with no layout and no `<body>` has no chrome to leave out, so
		// its `content` is the whole tree and `/mains` is checked under `main`
		for (path, root) in [
			("/bare/post", "main"),
			("/bare/post", "content"),
			("/mains", "main"),
			("/body", "main"),
			("/body", "content"),
			("/plain", "main"),
			("/plain", "content"),
		] {
			for target in targets.clone() {
				get(&mut world, router, at(path, root), target.clone())
					.await
					.xpect_contains("page")
					.xnot()
					.xpect_contains("Chrome")
					.xnot()
					.xpect_contains("deep");
			}
		}
	}

	/// A direct caller naming a target receives exactly the body a request
	/// for that media type answers.
	#[beet_core::test]
	async fn scope_equals_the_request() {
		let (mut world, router) = site();
		for root in ROOTS {
			for media_type in
				[MediaType::Html, MediaType::Markdown, MediaType::Text]
			{
				let direct = scoped(
					&mut world,
					router,
					post_at(root),
					media_type.clone(),
				)
				.await;
				get(&mut world, router, post_at(root), media_type)
					.await
					.xpect_eq(direct);
			}
		}
	}

	/// The scope releases the page whether its closure succeeds or fails, and
	/// a request leaves nothing behind either.
	#[beet_core::test]
	async fn releases_the_page() {
		let (mut world, router) = site();
		// the first render down each path spawns the cached systems and
		// observers it runs on
		get(&mut world, router, Request::get(POST), MediaType::Html).await;
		scoped(&mut world, router, Request::get(POST), MediaType::Html).await;
		let baseline = entities(&mut world);
		scoped(&mut world, router, at(POST, "main"), MediaType::Html).await;
		entities(&mut world).xpect_eq(baseline);
		world
			.run_async_then(async move |world| {
				LivePage::scoped(
					&world.entity(router),
					Request::get(POST),
					async |_live| -> Result<()> {
						bevybail!("the caller failed")
					},
				)
				.await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("the caller failed");
		entities(&mut world).xpect_eq(baseline);
		get(&mut world, router, at(POST, "content"), MediaType::Html).await;
		entities(&mut world).xpect_eq(baseline);
	}

	/// A target registered from outside `beet_ui` answers a request through
	/// the same registry the built-ins do.
	#[beet_core::test]
	async fn renders_a_downstream_target() {
		#[derive(Clone)]
		struct Shout;
		impl NodeRenderer for Shout {
			fn render(
				&mut self,
				cx: &mut RenderContext,
			) -> Result<MediaBytes, RenderError> {
				let mut text = PlainTextRenderer::default();
				cx.walk(&mut text);
				MediaBytes::new_string(
					MediaType::other("text/x-shout"),
					text.into_string().to_uppercase(),
				)
				.xok()
			}
		}
		impl RenderTarget for Shout {
			fn media_types(&self) -> Vec<MediaType> {
				vec![MediaType::other("text/x-shout")]
			}
		}
		let (mut world, router) = site();
		world.resource_mut::<RenderTargets>().register(Shout);
		get(
			&mut world,
			router,
			at(POST, "content"),
			MediaType::other("text/x-shout"),
		)
		.await
		.xpect_eq("PAGE\n");
	}
}
