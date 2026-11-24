//! Server islands: a named region of a page whose content renders live, so a
//! static, edge cached page shows live content without booting a runtime.
//!
//! An island contains its content, held unbuilt, and is never a route of its
//! own. It is addressed by its page and its name, `/_server-islands/<page
//! path>/<name>` (the home page's `/_server-islands/<name>`), which the one
//! [`ServerIslands`] route at the router root answers by building the page as
//! its own route does and rendering the island alone.
//!
//! ```bsx
//! <ServerIsland name="visits" {CacheHeaders{edge_max_age: "60s"}}>
//! 	<VisitCount/>
//! 	<p slot="fallback">Visits are counted live.</p>
//! </ServerIsland>
//! ```
//!
//! One declaration renders three ways, the caller's choice
//! (the island step's module docs):
//!
//! - The http page handler's html leaves each island a placeholder:
//!   `<beet-island name="<name>">` around the fallback, a custom element so it
//!   sits in a paragraph as well as a block, then a fixed inline script that fetches the island's html and swaps it in. The content
//!   never builds, so a page the edge holds for a week holds nothing live.
//! - The island route builds the one island inline and answers it alone, led
//!   by a `<style>` of the rules its own subtree selects, with a `beet-island`
//!   header naming it, which the script checks before it swaps.
//! - A page painted into a browser tab resolves placeholders too, the page
//!   it adopts having been served with them: the wrapper is
//!   [`ForeignChildren`], so the tab adopts it whole, whatever the script
//!   already swapped in, and a page the tab paints fresh, which runs no
//!   script, swaps each island in itself ([`IslandPlaceholder::swap`]).
//! - Every other render (a live surface, a scoped render, a non-html
//!   response) builds every island inline, like any other content.
//!
//! The island route's response carries the island's own [`CacheHeaders`],
//! `no-store` when it has none, never the page's or the router's.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A named region of its page whose content renders live, held unbuilt until
/// its page's preparation resolves it. See the [module docs](self).
///
/// In BSX the tag's children are its content and a `slot="fallback"` child
/// its fallback; in `rsx!` both are closures:
///
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// let island = rsx! {
/// 	<ServerIsland
/// 		name="visits"
/// 		content={|| rsx! { <span>"42 visits"</span> }}
/// 		fallback={|| rsx! { <span>"counting"</span> }}
/// 	/>
/// };
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default, HoldChildren)]
pub struct ServerIsland {
	/// The island's name, one path segment unique on its page and the last
	/// segment of its address.
	pub name: SmolStr,
	/// The content rendered live, built only when the island resolves.
	pub content: HeldTemplate,
	/// What the placeholder shows until the island swaps in, and for good with
	/// scripts off. Empty by default.
	pub fallback: HeldTemplate,
}

impl HoldChildren for ServerIsland {
	const SLOTS: &'static [(Option<&'static str>, &'static str)] =
		&[(None, "content"), (Some("fallback"), "fallback")];
}

impl ServerIsland {
	/// The reserved first segment of every island's address.
	pub const PREFIX: &str = "_server-islands";

	/// The response header naming the island a response renders, which the
	/// swap script requires before it swaps.
	pub const HEADER: &str = "beet-island";

	/// The inline script after each placeholder: read the island's name off
	/// the wrapper before it, fetch the island's html at its address, and swap
	/// it into the wrapper only when the response names the island, leaving
	/// the fallback otherwise. A fixed string, so no route data is ever
	/// interpolated into a script.
	pub(crate) const SWAP_SCRIPT: &str = concat!(
		"(()=>{",
		"const wrapper=document.currentScript.previousElementSibling;",
		"const name=wrapper.getAttribute(\"name\");",
		"const page=location.pathname.replace(/\\/$/,\"\");",
		"fetch(\"/_server-islands\"+page+\"/\"+name,{headers:{accept:\"text/html\"}})",
		".then(res=>res.ok&&res.headers.get(\"beet-island\")===name?res.text():null)",
		".then(html=>{if(html!==null)wrapper.innerHTML=html;})",
		".catch(()=>{});",
		"})()"
	);

	/// The address of the island `name` on the page at `page`, ie
	/// `/_server-islands/blog/ecs-router/comments`.
	pub fn address(page: &[SmolStr], name: &str) -> String {
		core::iter::once(Self::PREFIX)
			.chain(page.iter().map(SmolStr::as_str))
			.chain(core::iter::once(name))
			.fold(String::new(), |path, segment| path + "/" + segment)
	}

	/// Refuse a name that is not one non-empty path segment.
	pub(crate) fn assert_name(name: &str) -> Result {
		match name.is_empty() || name.contains('/') {
			false => Ok(()),
			true => bevybail!(
				"a server island's name is one path segment, the last of its \
				 address, not `{name}`"
			),
		}
	}
}

/// The `<beet-island>` wrapper of an island's placeholder: which island it
/// stands in for and where that island is fetched from, so a fresh paint in
/// a browser tab, which runs no script, swaps the island in as the swap
/// script would ([`Self::swap`]).
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub(crate) struct IslandPlaceholder {
	/// The island's name, which the response must name before it swaps.
	pub name: SmolStr,
	/// The island's address, ie `/_server-islands/blog/ecs-router/comments`.
	pub address: String,
}

impl IslandPlaceholder {
	/// Fetch the island and set `wrapper`'s html to it when the response
	/// names the island, leaving the fallback otherwise: the
	/// [`SWAP_SCRIPT`](ServerIsland::SWAP_SCRIPT), run from the world.
	#[cfg(target_arch = "wasm32")]
	pub(crate) async fn swap(
		self,
		wrapper: beet_core::exports::web_sys::Element,
	) {
		let response = match Request::get(&self.address)
			.with_header::<header::Accept>(vec![MediaType::Html])
			.send()
			.await
		{
			Ok(response) => response,
			Err(err) => {
				debug!(
					"server island `{}` kept its fallback: {err}",
					self.name
				);
				return;
			}
		};
		let named = response.parts.headers.first_raw(ServerIsland::HEADER)
			== Some(self.name.as_str());
		if !response.status().is_ok() || !named {
			return;
		}
		match response.text().await {
			Ok(html) => wrapper.set_inner_html(&html),
			Err(err) => {
				debug!("server island `{}` kept its fallback: {err}", self.name)
			}
		}
	}
}

/// The greedy capture the island route takes its address into: the page's
/// path, then the island's name.
const ISLAND_PATH_PARAM: &str = "island_path";

/// Marks the route [`ServerIslands`] spawns, so a placeholder can tell its
/// router answers islands rather than a catch-all matching the prefix.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ServerIslandsRoute;

/// The island route, `/_server-islands/<page path>/<name>`: builds the page
/// at the path as its own route does and answers the named [`ServerIsland`]
/// alone, as the request accepts.
///
/// Declared once at a router's root, like [`Sitemap`]. A glob with no
/// [`PageRoute`], so the sitemap, the route index, syndication and static
/// export never list it. A path no page matches, or a page with no island of
/// that name, is a `404`.
#[template]
pub fn ServerIslands() -> impl Bundle {
	(
		route::exchange(
			&format!("{}/*{ISLAND_PATH_PARAM}", ServerIsland::PREFIX),
			Action::<Request, Response>::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let route = cx.caller.clone();
					respond_island(route, cx.take()).await
				},
			),
		),
		HttpMethod::Get,
		ServerIslandsRoute,
	)
}

/// Answer the island `request` addresses, matched from the island `route`.
async fn respond_island(
	route: AsyncEntity,
	mut request: Request,
) -> Result<Response> {
	let segments = request
		.get_params(ISLAND_PATH_PARAM)
		.cloned()
		.unwrap_or_default();
	let Some((name, page)) = segments.split_last() else {
		return Err(HttpError::not_found().into());
	};
	let (name, page) = (name.clone(), page.to_vec());
	ServerIsland::assert_name(&name)?;
	// the page's own request: its path, never the island route's capture
	request.params_mut().remove(ISLAND_PATH_PARAM);
	request.url_mut().set_path(page.clone());
	let not_found = || {
		HttpError::new(
			StatusCode::NOT_FOUND,
			format!("no page at `/{}` to hold an island", page.join("/")),
		)
	};
	let node = LivePage::find_route(&route, page.clone())
		.await?
		.filter(ActionNode::is_scene)
		.ok_or_else(not_found)?;
	if hides_draft(route.world(), node.entity, BootstrapConfig::get().is_prod())
		.await
	{
		return Err(not_found().into());
	}
	let media_type =
		LivePage::negotiate(route.world(), request.parts()).await?;
	let scope = IslandScope::Only(name.clone());
	let (bytes, policy) =
		LivePage::prepare_route_with(&route, node, request, scope)
			.await?
			.release_after(async |live| {
				let bytes = live.render(&media_type).await?;
				let policy = live.root_component::<CacheHeaders>().await;
				(bytes, policy).xok()
			})
			.await?;
	let cache_control = policy
		.unwrap_or_else(CacheHeaders::no_store)
		.value(Some(media_type));
	let mut response = Response::ok().with_media(bytes);
	response
		.parts
		.headers
		.set_raw(ServerIsland::HEADER, name.to_string());
	response
		.parts
		.headers
		.set::<header::CacheControl>(cache_control);
	response.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;
	use beet_ui::style::Length;
	use beet_ui::style::common_props::MaxWidth;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	/// The page it renders on, read off the request context as a layout
	/// widget reads it.
	#[template(system)]
	fn PagePath(stack: Res<RequestContextStack>) -> impl Bundle {
		let path = stack.current().current_path();
		rsx! { <span>"island on /"{path}</span> }
	}

	/// A document shell carrying an island of its own beside the page.
	#[template]
	fn IslandShell() -> impl Bundle {
		rsx! {
			<html><body>
				<main><slot/></main>
				<ServerIsland name="where" content={|| rsx! { <PagePath/> }}/>
			</body></html>
		}
	}

	/// The island named `name`, its content `text` in a styled span, counting
	/// each build of it into `builds`, with a fallback and a minute's policy.
	fn island(
		name: &str,
		text: &'static str,
		builds: &Arc<AtomicUsize>,
	) -> Snippet {
		let builds = builds.clone();
		rsx! {
			<ServerIsland
				name={name}
				content={move || {
					builds.fetch_add(1, Ordering::SeqCst);
					rsx! { <span {inline_class![(MaxWidth, Length::Px(42.))]}>{text}</span> }
				}}
				fallback={|| rsx! { <em>"counting"</em> }}
				{CacheHeaders { edge_max_age: Duration::from_secs(60), ..default() }}
			/>
		}
	}

	/// A page route at `path` holding `body`.
	fn page_with(
		path: &str,
		body: impl 'static + Send + Sync + Clone + Fn() -> Snippet,
	) -> impl Bundle {
		render_action::fixed_func_route(path, move || {
			let body = body();
			rsx! { <article>{body}</article> }
		})
	}

	/// The fixture: a home page and a post each holding a `visits` island, a
	/// page living at the post's path plus `visits` holding its own, a
	/// `docs/intro` page under [`IslandShell`], and the island route, under a
	/// router edge caching html for seven days.
	fn site(builds: &Arc<AtomicUsize>) -> (World, Entity) {
		let mut app = App::new().with_plugins((AsyncPlugin, RouterPlugin));
		app.install_type::<(PagePath, IslandShell)>();
		let mut world = core::mem::take(app.world_mut());
		let (home, post, nested) =
			(builds.clone(), builds.clone(), builds.clone());
		let router = world
			.spawn((
				Router,
				CacheHeaders::default(),
				CacheHeadersMiddleware,
				children![
					page_with("", move || island(
						"visits",
						"home visits",
						&home
					)),
					page_with("blog/post", move || island(
						"visits",
						"post visits",
						&post
					)),
					page_with("blog/post/visits", move || {
						island("visits", "nested visits", &nested)
					}),
					(
						PathPartial::new("docs"),
						Layout::of::<IslandShell>(),
						children![page_with(
							"intro",
							|| rsx! { <p>"intro"</p> }
						)]
					),
					rsx! { <ServerIslands/> },
				],
			))
			.flush();
		(world, router)
	}

	async fn get(
		world: &mut World,
		router: Entity,
		path: &str,
		accept: MediaType,
	) -> Response {
		world
			.entity_mut(router)
			.exchange(Request::get(path).with_accept(accept))
			.await
	}

	async fn html(world: &mut World, router: Entity, path: &str) -> String {
		get(world, router, path, MediaType::Html)
			.await
			.unwrap_str()
			.await
	}

	/// The error `path` answers, as its body reads.
	async fn error(
		world: &mut World,
		router: Entity,
		path: &str,
		accept: MediaType,
	) -> String {
		get(world, router, path, accept)
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
	}

	/// The island address [`ServerIsland::address`] composes.
	#[beet_core::test]
	fn composes_the_address() {
		ServerIsland::address(&[], "visits")
			.xpect_eq("/_server-islands/visits");
		ServerIsland::address(&["blog".into(), "post".into()], "comments")
			.xpect_eq("/_server-islands/blog/post/comments");
	}

	/// The whole page's html carries the wrapper, its fallback and the swap
	/// script, and builds none of the island's content.
	#[beet_core::test]
	async fn page_renders_the_placeholder() {
		let builds = Arc::new(AtomicUsize::new(0));
		let (mut world, router) = site(&builds);
		html(&mut world, router, "/blog/post")
			.await
			.xpect_contains(
				"<beet-island name=\"visits\"><em>counting</em></beet-island><script>",
			)
			.xpect_contains("document.currentScript.previousElementSibling")
			.xnot()
			.xpect_contains("post visits");
		builds.load(Ordering::SeqCst).xpect_eq(0);
	}

	/// The island route answers the island alone, led by the rules its own
	/// subtree selects and named in its header, for a nested page and the
	/// root page, in a world that never built the island before.
	#[beet_core::test]
	async fn route_renders_the_island_alone() {
		let builds = Arc::new(AtomicUsize::new(0));
		let (mut world, router) = site(&builds);
		for (path, text) in [
			("/_server-islands/blog/post/visits", "post visits"),
			("/_server-islands/visits", "home visits"),
		] {
			let response = get(&mut world, router, path, MediaType::Html).await;
			response
				.parts
				.headers
				.get_raw(ServerIsland::HEADER)
				.unwrap()
				.xpect_eq(vec!["visits".to_string()]);
			response
				.unwrap_str()
				.await
				.xpect_starts_with("<style>")
				.xpect_contains("max-width: 42px")
				.xpect_contains(text)
				.xnot()
				.xpect_contains("counting")
				.xnot()
				.xpect_contains("<article>")
				.xnot()
				.xpect_contains("<script>");
		}
		builds.load(Ordering::SeqCst).xpect_eq(2);
	}

	/// The prefix, not the shape of the rest, marks an island: a page living
	/// at a path ending in another page's island name keeps its own apart.
	#[beet_core::test]
	async fn nested_page_keeps_its_islands_apart() {
		let (mut world, router) = site(&default());
		html(&mut world, router, "/_server-islands/blog/post/visits")
			.await
			.xpect_contains("post visits");
		html(
			&mut world,
			router,
			"/_server-islands/blog/post/visits/visits",
		)
		.await
		.xpect_contains("nested visits");
	}

	/// An island in a layout builds in its page's request context.
	#[beet_core::test]
	async fn layout_island_reads_its_page() {
		let (mut world, router) = site(&default());
		html(&mut world, router, "/_server-islands/docs/intro/where")
			.await
			.xpect_contains("island on /docs/intro");
		html(&mut world, router, "/docs/intro")
			.await
			.xpect_contains("<beet-island name=\"where\"></beet-island>")
			.xnot()
			.xpect_contains("island on");
	}

	/// A text render is never edge cached as html, so it carries the island
	/// inline, as does charcell.
	#[beet_core::test]
	async fn text_renders_inline() {
		let (mut world, router) = site(&default());
		for accept in [MediaType::Text, MediaType::AnsiTerm] {
			get(&mut world, router, "/blog/post", accept)
				.await
				.unwrap_str()
				.await
				.xpect_contains("post visits")
				.xnot()
				.xpect_contains("counting");
		}
	}

	/// An island answers its own policy, never the page's seven days, and
	/// `no-store` when it declares none.
	#[beet_core::test]
	async fn island_answers_its_own_policy() {
		let (mut world, router) = site(&default());
		let cache_control = async |world: &mut World, path: &str| {
			get(world, router, path, MediaType::Html)
				.await
				.parts
				.headers
				.get::<header::CacheControl>()
				.unwrap()
				.unwrap()
		};
		cache_control(&mut world, "/blog/post")
			.await
			.xpect_eq("public, max-age=0, must-revalidate, s-maxage=604800");
		cache_control(&mut world, "/_server-islands/blog/post/visits")
			.await
			.xpect_eq("public, max-age=0, must-revalidate, s-maxage=60");
		cache_control(&mut world, "/_server-islands/docs/intro/where")
			.await
			.xpect_eq("no-cache, no-store, must-revalidate");
	}

	/// An unknown island or page is a `404`.
	#[beet_core::test]
	async fn unknown_island_or_page_is_not_found() {
		let (mut world, router) = site(&default());
		for path in [
			"/_server-islands/blog/post/comments",
			"/_server-islands/nope/visits",
		] {
			get(&mut world, router, path, MediaType::Html)
				.await
				.status()
				.xpect_eq(StatusCode::NOT_FOUND);
		}
	}

	/// A router serving `body` at `/` beside the island route.
	fn serving(
		body: impl 'static + Send + Sync + Clone + Fn() -> Snippet,
	) -> (World, Entity) {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let router = world
			.spawn((Router, children![
				page_with("", body),
				rsx! { <ServerIslands/> }
			]))
			.flush();
		(world, router)
	}

	/// Two islands sharing a name, island content carrying a script and a
	/// placeholder under a router answering no islands are errors.
	#[beet_core::test]
	async fn malformed_islands_fail() {
		let builds = Arc::new(AtomicUsize::new(0));
		let twice = builds.clone();
		let (mut world, router) = serving(move || {
			let (first, second) =
				(island("visits", "a", &twice), island("visits", "b", &twice));
			rsx! { {first}{second} }
		});
		error(&mut world, router, "/", MediaType::Html)
			.await
			.xpect_contains(
				"two server islands on one page are named `visits`",
			);

		let (mut world, router) = serving(|| {
			rsx! {
				<ServerIsland name="tracker" content={|| rsx! { <script>"track()"</script> }}/>
			}
		});
		error(
			&mut world,
			router,
			"/_server-islands/tracker",
			MediaType::Html,
		)
		.await
		.xpect_contains("server island `tracker` contains a `<script>`");
		error(&mut world, router, "/", MediaType::Text)
			.await
			.xpect_contains("contains a `<script>`");

		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let lone = builds.clone();
		let router = world
			.spawn((Router, children![page_with("", move || island(
				"visits", "a", &lone
			))]))
			.flush();
		error(&mut world, router, "/", MediaType::Html)
			.await
			.xpect_contains("declares no `<ServerIslands/>`");
	}

	/// Content arriving asynchronously, ie a store read, lands before the
	/// island renders.
	#[beet_core::test]
	async fn awaits_async_content() {
		let (mut world, router) = serving(|| {
			rsx! {
				<ServerIsland name="late" content={|| Snippet::from_bundle(OnSpawn::new(
					|entity: &mut EntityWorldMut| {
						TemplatePending::park_task(
							entity,
							PendingKind::Passive,
							"late content",
							|entity, guard| async move {
								async_ext::yield_now().await;
								let parent = entity.id();
								entity
									.world()
									.with(move |world: &mut World| {
										world.spawn((ChildOf(parent), rsx! {
											<span>"arrived"</span>
										}));
										guard.resolve(world);
									})
									.await;
							},
						)
						.unwrap();
					},
				))}/>
			}
		});
		html(&mut world, router, "/_server-islands/late")
			.await
			.xpect_contains("<span>arrived</span>");
	}

	/// Static export writes the placeholder and never walks the island route.
	#[beet_core::test]
	async fn static_export_writes_the_placeholder() {
		let (mut world, router) = site(&default());
		let out = BlobStore::temp();
		let store = out.clone();
		let written = world
			.run_async_then(async move |world| {
				StaticExport::export(&world, router, &store).await
			})
			.await
			.unwrap();
		written
			.iter()
			.any(|path| path.starts_with(ServerIsland::PREFIX))
			.xpect_false();
		out.get(&RelPath::new("blog/post/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("<beet-island name=\"visits\">")
			.xnot()
			.xpect_contains("post visits");
	}

	/// In markup the tag's children are its content and a `slot="fallback"`
	/// child its fallback, held unbuilt, its spreads applied like any tag's.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	async fn markup_holds_its_children() {
		const MARKUP: &str = r#"<article><ServerIsland name="visits" {CacheHeaders{edge_max_age: "60s"}}>
			<span>live</span>
			<em slot="fallback">later</em>
		</ServerIsland></article>"#;
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let router = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || {
					OnSpawn::new(|entity: &mut EntityWorldMut| {
						entity
							.insert_template(
								SceneBody::parse_bsx(entity.world(), MARKUP)
									.unwrap(),
							)
							.unwrap();
					})
				}),
				rsx! { <ServerIslands/> },
			]))
			.flush();
		html(&mut world, router, "/")
			.await
			.xpect_contains(
				"<beet-island name=\"visits\"><em>later</em></beet-island>",
			)
			.xnot()
			.xpect_contains("live");
		let response = get(
			&mut world,
			router,
			"/_server-islands/visits",
			MediaType::Html,
		)
		.await;
		response
			.parts
			.headers
			.get::<header::CacheControl>()
			.unwrap()
			.unwrap()
			.xpect_contains("s-maxage=60");
		response
			.unwrap_str()
			.await
			.xpect_contains("<span>live</span>")
			.xnot()
			.xpect_contains("later");
	}
}
