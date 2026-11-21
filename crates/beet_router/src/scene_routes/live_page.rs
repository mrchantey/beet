//! A page built and held alive between its build and its renders.
//!
//! [`LivePage::prepare`] runs a built page's render middleware (the layouts),
//! the island step (each [`ServerIsland`] a placeholder, inline or the root),
//! resolves the `--root` cascade ([`RenderRoot`]) and, when the request names
//! a [`MediaIngestPolicy`], the media resolve step ([`InlineBlob`]). The live
//! page keeps the request it was built for, the [`RequestParts`] every render
//! of it answers, and renders through any registered target, or as any
//! registered lexicon's member, as many times as its holder likes; its release takes back everything the preparation put on
//! a tree it does not own. [`LivePage::scoped`] and [`LivePage::respond`]
//! release it whatever the outcome; a live surface binds one instead.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A page built and held alive: its render middleware run (the layouts), the
/// `--root` cascade resolved, the route's [`DocMeta`] carried onto the root it
/// chose, and the media its request named resolved. Rendered through any
/// registered target as many times as its holder likes, each render answering
/// the request it was built for.
///
/// [`LivePage::scoped`] and [`LivePage::respond`] release it whatever the
/// outcome, so an early `?` never strands a page; a live surface binds one
/// instead ([`into_surface`](Self::into_surface)).
#[derive(Clone)]
pub struct LivePage {
	world: AsyncWorld,
	/// The request the page was built for, which every render answers.
	request: RequestParts,
	/// The whole page, its layouts included, on which its ephemerals are
	/// recorded.
	page: Entity,
	/// The entity a render starts from, the cascade's answer to `--root`.
	root: Entity,
	/// The ephemeral entities released with the page.
	to_despawn: Vec<Entity>,
	/// The root a copy of the route's [`DocMeta`] was inserted on, taken back
	/// on release.
	carried_meta: Option<Entity>,
	/// The entities the media resolve step gave an [`InlineBlob`], taken back
	/// on release so a persistent tree keeps no fetched bytes.
	inline_blobs: Vec<Entity>,
}

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
	/// `route` and its ancestors (the layouts), resolve every
	/// [`ServerIsland`] inline, resolve the `--root` cascade ([`RenderRoot`]),
	/// carry the route's [`DocMeta`] onto the root it chose, so a scene
	/// response reads the page's metadata where it reads the tree, and, when
	/// the request names a [`MediaIngestPolicy`], run the media resolve step
	/// ([`InlineBlob::resolve`]). The live page keeps `parts`, the request
	/// every render of it answers. A failure releases what was built.
	pub async fn prepare(
		page: Entity,
		route: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Self> {
		Self::prepare_with(page, route, parts, IslandScope::Inline).await
	}

	/// [`prepare`](Self::prepare), resolving the page's islands as `islands`
	/// asks: the island step runs after the layouts and before the cascade, an
	/// island it makes the root winning over `--root`.
	pub(crate) async fn prepare_with(
		page: Entity,
		route: &AsyncEntity,
		parts: RequestParts,
		islands: IslandScope,
	) -> Result<Self> {
		let params = match RenderParams::of(&parts) {
			Ok(params) => params,
			Err(err) => {
				Self::discard(route.world(), page).await;
				return Err(err);
			}
		};
		let request = parts.clone();
		// apply ancestor render middleware (layout wrapping, etc.)
		let wrapped = match route
			.call_with_middleware(Action::new_fixed(page), parts.clone())
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
		// the whole tree, its ephemerals and the request context it renders in
		let found = world
			.with(
				move |world_mut: &mut World| -> Result<(Entity, Vec<Entity>, RequestContext)> {
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
					// the route's metadata lives on its content, the end of any
					// layout chain, which the resolved root generally is not
					let content = LayoutContent::terminal(world_mut, rendered)?;
					// the router a source fetch re-enters
					let router = world_mut
						.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
							trees.get_entity(route_id)
						})
						.unwrap_or(route_id);
					let cx = RequestContext::new(parts, content, route_id, router);
					(rendered, to_despawn, cx).xok()
				},
			)
			.await;
		let (rendered, mut to_despawn, cx) = match found {
			Ok(found) => found,
			Err(err) => {
				Self::discard(&world, wrapped).await;
				return Err(err);
			}
		};
		let (content, router) = (cx.content(), cx.router());
		let islands = match islands.resolve(&world, rendered, cx).await {
			Ok(islands) => islands,
			Err(err) => {
				Self::discard(&world, wrapped).await;
				return Err(err);
			}
		};
		to_despawn.extend(islands.spawned);
		let island_root = islands.root;
		let live_world = world.clone();
		let mut live = world
			.with(move |world_mut: &mut World| {
				let root = island_root.unwrap_or_else(|| {
					world_mut.with_state::<RenderRootQuery, _>(|query| {
						query.resolve(rendered, params.root)
					})
				});
				// an island answers its own region, never the page's metadata
				let carried_meta = world_mut
					.get::<DocMeta>(content)
					.filter(|_| {
						island_root.is_none()
							&& !world_mut.entity(root).contains::<DocMeta>()
					})
					.cloned()
					.map(|meta| {
						world_mut.entity_mut(root).insert(meta);
						root
					});
				Self {
					world: live_world,
					request,
					page: wrapped,
					root,
					to_despawn,
					carried_meta,
					inline_blobs: Vec::new(),
				}
			})
			.await;
		// a request naming no policy fetches nothing
		let Some(policy) = params.media_ingest else {
			return live.xok();
		};
		match InlineBlob::resolve(
			&live.world,
			router,
			&live.request,
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
		let node = Self::find_route(router, request.path().clone())
			.await?
			.ok_or_else(|| {
				bevyhow!("no route matched {}", request.path_string())
			})?;
		Self::prepare_route(router, node, request).await
	}

	/// Build the scene route `node`, matched for `request` in the tree
	/// `router` sits in, and [`prepare`](Self::prepare) it.
	pub(crate) async fn prepare_route(
		router: &AsyncEntity,
		node: ActionNode,
		request: Request,
	) -> Result<Self> {
		Self::prepare_route_with(router, node, request, IslandScope::Inline)
			.await
	}

	/// [`prepare_route`](Self::prepare_route), resolving the page's islands
	/// as `islands` asks.
	pub(crate) async fn prepare_route_with(
		router: &AsyncEntity,
		node: ActionNode,
		request: Request,
		islands: IslandScope,
	) -> Result<Self> {
		let (route, page, parts) = Self::build(router, node, request).await?;
		Self::prepare_with(page, &route, parts, islands).await
	}

	/// The route `path` matches in the route tree `router` sits in, `None`
	/// for no match.
	pub(crate) async fn find_route(
		router: &AsyncEntity,
		path: Vec<SmolStr>,
	) -> Result<Option<ActionNode>> {
		router
			.with_state::<AncestorQuery<&RouteTree>, _>(move |entity, query| {
				query
					.get(entity)
					.map(|tree| tree.find(&path).cloned())
					.map_err(|_| {
						bevyhow!(
							"route tree not found, was the RouterPlugin added?"
						)
					})
			})
			.await?
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
	/// type from `Accept`, prepare the page, render it and release it,
	/// answering with the page's [`PageStatus`], else `200`.
	///
	/// This is the http page handler, so a request with no `Accept` or a
	/// wildcard (`*/*`) renders the web document, the registry's
	/// [`DEFAULT`](DocRenderers::DEFAULT). Negotiating before the build means
	/// a type nothing renders builds nothing and answers `406 Not Acceptable`.
	/// Html leaves each [`ServerIsland`] a placeholder, since the edge may
	/// hold it; any other type resolves them inline.
	pub async fn respond(
		page: Entity,
		route: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Response> {
		let media_type = match Self::negotiate(route.world(), &parts).await {
			Ok(media_type) => media_type,
			Err(err) => {
				Self::discard(route.world(), page).await;
				return Err(err);
			}
		};
		let islands = match media_type == MediaType::Html {
			true => IslandScope::Placeholders,
			false => IslandScope::Inline,
		};
		// read before the render releases the page
		let status = route
			.world()
			.with(move |world: &mut World| {
				world
					.get::<PageStatus>(page)
					.map_or(StatusCode::OK, |status| status.0)
			})
			.await;
		let bytes = Self::prepare_with(page, route, parts, islands)
			.await?
			.release_after(async |live| live.render(&media_type).await)
			.await?;
		Response::ok().with_media(bytes).with_status(status).xok()
	}

	/// The media type `parts` negotiates from the registered render targets,
	/// a type nothing renders answering `406 Not Acceptable`.
	pub(crate) async fn negotiate(
		world: &AsyncWorld,
		parts: &RequestParts,
	) -> Result<MediaType> {
		let parts = parts.clone();
		world
			.with(move |world: &mut World| -> Result<MediaType> {
				world
					.get_resource::<DocRenderers>()
					.ok_or_else(|| {
						bevyhow!(
							"no `DocRenderers` in this world: add the `RenderPlugin`"
						)
					})?
					.negotiate(&parts)
					.map_err(HttpError::from)?
					.xok()
			})
			.await
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
	/// fetched onto the render root from `DocMeta::social_image_url`.
	pub async fn cover(&self) -> Option<InlineBlob> {
		self.root_component::<InlineBlob>().await
	}

	/// A copy of the render root's `C`, ie an island's [`CacheHeaders`].
	pub(crate) async fn root_component<C: Clone + Component>(
		&self,
	) -> Option<C> {
		let root = self.root;
		self.world
			.with(move |world: &mut World| world.get::<C>(root).cloned())
			.await
	}

	/// Render the page as `media_type`: this page's request, accepting only
	/// that type, so the body is exactly the one a client asking for it gets.
	pub async fn render(&self, media_type: &MediaType) -> Result<MediaBytes> {
		let root = self.root;
		let request = self.request.clone().with_accept(media_type.clone());
		self.world
			.with(move |world: &mut World| {
				DocRenderers::render(world, root, &request)
			})
			.await?
			.xok()
	}

	/// Render the page as a member of the lexicon def `nsid` through
	/// [`LexiconRenderers::render`], answering this page's request, as
	/// [`render`](Self::render) does a media type, so a publish step renders a
	/// record's `content` off the same tree as its `textContent`.
	pub async fn render_lexicon(&self, nsid: &Nsid) -> Result<OpenUnion> {
		let (root, request, nsid) =
			(self.root, self.request.clone(), nsid.clone());
		self.world
			.with(move |world: &mut World| {
				LexiconRenderers::render(world, root, &request, &nsid)
			})
			.await
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

	/// The content the scene route `node` builds for `request`, matched in
	/// the tree `router` sits in, no middleware applied: ie a card deck
	/// counting or gathering its routes' cards. The caller releases it with
	/// [`despawn_content`](Self::despawn_content).
	pub(crate) async fn build_content(
		router: &AsyncEntity,
		node: ActionNode,
		request: Request,
	) -> Result<Entity> {
		Self::build(router, node, request)
			.await
			.map(|(_, page, _)| page)
	}

	/// Releases content [`build_content`](Self::build_content) built: its
	/// ephemerals, itself among them.
	pub(crate) fn despawn_content(world: &mut World, page: Entity) {
		let to_despawn = world
			.get_entity(page)
			.ok()
			.and_then(|entity| entity.get::<DespawnAfterRender>())
			.map(|despawn| despawn.0.clone())
			.unwrap_or_else(|| vec![page]);
		Self::despawn(world, to_despawn);
	}

	/// Build the scene route `node` matched for `request`, answering the
	/// route, the page it built and the request's parts with the matched path
	/// params merged in.
	async fn build(
		router: &AsyncEntity,
		node: ActionNode,
		mut request: Request,
	) -> Result<(AsyncEntity, Entity, RequestParts)> {
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
	pub(crate) async fn release_after<T, Fut>(
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
	/// tree it does not own: the metadata and the fetched media.
	async fn release(self) {
		self.world
			.with(move |world: &mut World| {
				if let Some(entity) = self.carried_meta
					&& let Ok(mut entity) = world.get_entity_mut(entity)
				{
					entity.remove::<DocMeta>();
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
	/// leaves out what the article layout contributed. Asked for frontmatter,
	/// markdown leads the rung with the page's, so `content` reads back as the
	/// source it was parsed from.
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
		get(
			&mut world,
			router,
			at(POST, "content").with_param("frontmatter", "toml"),
			MediaType::Markdown,
		)
		.await
		.trim()
		.xpect_eq(
			"+++\ntitle = \"Full Stack Bevy\"\ndescription = \"all about Full Stack Bevy\"\ncreated = \"2025-07-11\"\nauthors = [\"Pete Hayman\"]\n+++\n\npage",
		);
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
	/// `DocMeta` carried onto that root at every rung.
	#[cfg(all(feature = "template_serde", feature = "json"))]
	#[beet_core::test]
	async fn scene_root_carries_the_doc_meta() {
		let (mut world, router) = site();
		for root in ROOTS {
			get(&mut world, router, post_at(root), MediaType::Json)
				.await
				.xpect_contains("DocMeta")
				.xpect_contains("Full Stack Bevy");
		}
		get(&mut world, router, at(POST, "content"), MediaType::Json)
			.await
			.xnot()
			.xpect_contains("\"h1\"");
	}

	/// The carried `DocMeta` sits on the very root a render starts from,
	/// whichever rung chose it, and goes when the page is released.
	#[beet_core::test]
	async fn carries_the_doc_meta_to_each_rung() {
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
										world.get::<DocMeta>(root).cloned(),
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

	/// A type no target renders answers `406 Not Acceptable`, building nothing.
	#[beet_core::test]
	async fn unrenderable_type_is_not_acceptable() {
		let (mut world, router) = site();
		// the first render spawns the cached systems and observers it runs on
		get(&mut world, router, Request::get(POST), MediaType::Html).await;
		let baseline = entities(&mut world);
		world
			.entity_mut(router)
			.exchange(Request::get(POST).with_accept(MediaType::Png))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_ACCEPTABLE);
		entities(&mut world).xpect_eq(baseline);
	}

	/// One live page renders as every type its holder asks for, each the body
	/// a request for that type answers.
	#[beet_core::test]
	async fn renders_one_page_as_two_types() {
		let (mut world, router) = site();
		let (html, markdown) = world
			.run_async_then(async move |world| {
				LivePage::scoped(
					&world.entity(router),
					at(POST, "main"),
					async |live| {
						(
							live.render(&MediaType::Html).await?.to_string(),
							live.render(&MediaType::Markdown)
								.await?
								.to_string(),
						)
							.xok()
					},
				)
				.await
			})
			.await
			.unwrap();
		html.clone().xpect_starts_with("<main>");
		markdown.clone().xpect_contains("# Full Stack Bevy");
		get(&mut world, router, at(POST, "main"), MediaType::Html)
			.await
			.xpect_eq(html);
		get(&mut world, router, at(POST, "main"), MediaType::Markdown)
			.await
			.xpect_eq(markdown);
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
	/// the same registry the built-ins do, reading the request it answers.
	#[beet_core::test]
	async fn renders_a_downstream_target() {
		#[derive(Clone)]
		struct Shout;
		impl DocRenderer for Shout {
			fn media_types(&self) -> Vec<MediaType> {
				vec![MediaType::other("text/vnd.shout")]
			}


			fn render(
				&mut self,
				cx: &mut RenderContext,
			) -> Result<MediaBytes, RenderError> {
				let mut text = PlainTextRenderer::default();
				cx.walk(&mut text);
				MediaBytes::new_string(
					MediaType::other("text/vnd.shout"),
					format!(
						"{}: {}",
						cx.request.path_string(),
						text.into_string().to_uppercase()
					),
				)
				.xok()
			}
		}
		impl Installer for Shout {
			fn install(self, app: &mut App) {
				DocRenderers::register_on(app, self);
			}
		}
		let (mut world, router) = site();
		world.resource_mut::<DocRenderers>().register(Shout);
		get(
			&mut world,
			router,
			at(POST, "content"),
			MediaType::other("text/vnd.shout"),
		)
		.await
		.xpect_eq("/blog/full-stack-bevy: PAGE\n");
	}
}
