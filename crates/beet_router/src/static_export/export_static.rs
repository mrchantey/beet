//! Static-site export driven by the runtime [`RouteTree`].
//!
//! Walks the router's route tree for every static-path scene route or route
//! marked [`ExportStrategy::Static`], renders each through the same dispatch path
//! a live request would take, and writes the resulting HTML to an output
//! [`BlobStore`] as `<path>/index.html`.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Static-site export: collect the qualifying routes, render each, write the
/// resulting HTML to a [`BlobStore`].
pub struct StaticExport;

impl StaticExport {
	/// Collects the static-route paths a no-code site ships, in route-tree order.
	///
	/// See [`exports`](Self::exports) for which routes qualify. Shared by
	/// [`collect_static_html`] and the `export-pdf` command, so both ship exactly
	/// the same page set.
	pub async fn collect_paths(
		world: &AsyncWorld,
		router: Entity,
	) -> Result<Vec<SmolPath>> {
		// drafts are excluded only in production; the process stage defaults to
		// dev (keep drafts) when neither transport named one.
		let is_prod = BootstrapConfig::get().is_prod();
		world
			.with(move |world: &mut World| Self::paths(world, router, is_prod))
			.await
	}

	/// [`collect_paths`](Self::collect_paths) against a borrowed world and an
	/// explicit stage, ie the whole walk minus the process-config read. The prod
	/// path is only reachable here, since the process stage is an immutable
	/// global.
	pub(crate) fn paths(
		world: &World,
		router: Entity,
		is_prod: bool,
	) -> Result<Vec<SmolPath>> {
		RouteTree::of(world, router)?
			.clone()
			.flatten_nodes()
			.into_iter()
			.filter(|node| Self::exports(world, node, is_prod))
			.map(|node| node.path.annotated_path())
			.collect::<Vec<_>>()
			.xok()
	}

	/// Whether `node` ships in a static export: its path is fully static, its
	/// method is `GET`, and it is either a scene route or marked
	/// [`ExportStrategy::Static`].
	///
	/// A page additionally passes [`ActionNode::is_public_page`], the shared
	/// visibility rule the sitemap and the feeds read, so a draft ships to a
	/// dev/staging preview and never to production. The non-page half stays
	/// here: the runtime assets a self-contained export needs (eg
	/// `js/reactivity.js`) are not pages and are listed nowhere.
	fn exports(world: &World, node: &ActionNode, is_prod: bool) -> bool {
		if !node.path.is_static() {
			return false;
		}
		if node.method.is_some_and(|method| method != HttpMethod::Get) {
			return false;
		}
		let entity = world.entity(node.entity);
		if node.is_page_route
			&& !node.is_public_page(entity.get::<PageMeta>(), is_prod)
		{
			return false;
		}
		node.is_scene()
			|| entity.get::<ExportStrategy>().copied().unwrap_or_default()
				== ExportStrategy::Static
	}
}
/// Renders every static route in the router to HTML, in route-tree order. See
/// [`StaticExport::exports`] for which routes qualify.
async fn collect_static_html(
	world: &AsyncWorld,
	router: Entity,
) -> Result<Vec<(SmolPath, String)>> {
	let paths = StaticExport::collect_paths(world, router).await?;
	// the tree lives on the entry root, the dispatch on the router beneath it
	let entity = world
		.run_system_cached_with::<_, Result<Entity>, _, _>(find_router, router)
		.await
		.map_err(BevyError::from)
		.flatten()
		.map(|router| world.entity(router))?;
	let mut pages = Vec::new();
	for path in paths {
		let request = Request::get(path.with_leading_slash())
			.with_accept(MediaType::Html);
		let response = entity.exchange(request).await;
		let html = response
			.into_result()
			.await
			.map_err(|err| bevyhow!("failed to render '{path}': {err}"))?
			.text()
			.await?;
		pages.push((path, html));
	}
	Ok(pages)
}

impl StaticExport {
	/// Renders every static route and writes it to the output store, returning the
	/// written paths. A page writes to `<path>/index.html` (clean URLs); an asset
	/// route with a file extension (eg `js/reactivity.js`) writes its raw file, so
	/// the `<script src="/js/reactivity.js">` a reactive page references resolves and
	/// the export is self-contained.
	pub async fn export(
		world: &AsyncWorld,
		router: Entity,
		out: &BlobStore,
	) -> Result<Vec<SmolPath>> {
		let mut pages = collect_static_html(world, router).await?;
		pages.extend(
			world
				.with(move |world: &mut World| Self::redirects(world, router))
				.await?,
		);
		let mut written = Vec::new();
		for (path, html) in pages {
			let out_path = if path.segments().is_empty() {
				SmolPath::new("index.html")
			} else if path.extension().is_some() {
				path
			} else {
				path.join("index.html")
			};
			out.insert(&out_path, html).await?;
			written.push(out_path);
		}
		Ok(written)
	}

	/// The meta-refresh stub each [`Redirect`] route in the tree exports as,
	/// paired with the path it serves at.
	///
	/// A static host serves files, not redirects, so the old url would 404 for
	/// exactly the readers a redirect exists for: everyone who followed a link
	/// published before the rename. The stub is the file that behaves like one
	/// — an immediate refresh, a canonical link so the crawler consolidates
	/// onto the new url rather than indexing the stub, and a plain anchor for
	/// whoever has neither. Live serving is untouched and still answers a 301.
	fn redirects(
		world: &mut World,
		router: Entity,
	) -> Result<Vec<(SmolPath, String)>> {
		let package = world.get_resource::<PackageConfig>().cloned();
		RouteTree::of(world, router)?
			.clone()
			.flatten_nodes()
			.into_iter()
			.filter(|node| node.path.is_static())
			.filter_map(|node| {
				let target = world
					.entity(node.entity)
					.get::<RedirectTo>()?
					.location(&node.path);
				// the canonical url must be absolute to consolidate anything,
				// so a site naming no origin gets the stub without one
				let canonical = package
					.as_ref()
					.and_then(|package| package.absolute_url(&target).ok());
				Some((
					node.path.annotated_path(),
					redirect_stub(&target, canonical.as_deref()),
				))
			})
			.collect::<Vec<_>>()
			.xok()
	}
}

/// The body of a redirect stub: a zero-delay refresh, the canonical target, and
/// a link for a reader whose browser honours neither.
fn redirect_stub(target: &str, canonical: Option<&str>) -> String {
	let canonical = canonical
		.map(|url| format!("<link rel=\"canonical\" href=\"{url}\">\n"))
		.unwrap_or_default();
	format!(
		"<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
		 <meta http-equiv=\"refresh\" content=\"0; url={target}\">\n{canonical}\
		 <title>Redirecting</title>\n</head>\n\
		 <body><a href=\"{target}\">Continue to {target}</a></body>\n</html>\n"
	)
}
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	#[beet_core::test]
	async fn exports_static_scenes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// `Router::with_defaults` also wires the std-only `/app-info` scene route, which
		// reads a `PackageConfig` at render; insert one so it exports cleanly.
		world.insert_resource(pkg_config!());
		let router = world
			.spawn((Router::with_defaults(), children![
				(
					render_action::fixed_func_route(
						"about",
						|| rsx! { <p>"About"</p> }
					),
					HttpMethod::Get
				),
				(
					render_action::fixed_func_route(
						"",
						|| rsx! { <h1>"Home"</h1> }
					),
					HttpMethod::Get
				),
			]))
			.flush();

		let out = BlobStore::temp();
		let out2 = out.clone();
		let written = world
			.run_async_then(async move |world| {
				StaticExport::export(&world, router, &out2).await
			})
			.await
			.unwrap();

		// the two user scene routes plus the `app-info` scene and the
		// `js/reactivity.js` runtime asset, both wired by `Router::with_defaults`.
		written.len().xpect_eq(4);
		out.get(&SmolPath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("Home");
		out.get(&SmolPath::new("about/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("About");
		// the runtime asset is a raw file (not `<path>/index.html`), so a reactive
		// page's `<script src="/js/reactivity.js">` resolves: a self-contained export.
		out.get(&SmolPath::new("js/reactivity.js"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("class EntityMut");
		out.get(&SmolPath::new("app-info/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("App Info");
	}

	/// A syndication route is an ordinary static route, so it needs nothing of
	/// the export pipeline: its path carries an extension, so it lands verbatim
	/// at `sitemap.xml` rather than as `sitemap.xml/index.html`, with the body
	/// the live site serves.
	#[beet_core::test]
	async fn exports_syndication_routes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			homepage: Some("https://beet.org".into()),
			..pkg_config!()
		});
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Sitemap/>
				<Robots/>
			})))
			.unwrap();
		world.spawn((
			ChildOf(root),
			render_action::fixed_func_route(
				"about",
				|| rsx! { <p>"About"</p> },
			),
			HttpMethod::Get,
			PageRoute,
		));
		world.flush();

		let out = BlobStore::temp();
		let out2 = out.clone();
		let written = world
			.run_async_then(async move |world| {
				StaticExport::export(&world, root, &out2).await
			})
			.await
			.unwrap();
		exported(&written, "sitemap.xml").xpect_true();

		let read = async |path: &str| {
			out.get(&SmolPath::new(path))
				.await
				.unwrap()
				.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
		};
		read("sitemap.xml")
			.await
			.xpect_contains("<loc>https://beet.org/about</loc>");
		read("robots.txt")
			.await
			.xpect_contains("Sitemap: https://beet.org/sitemap.xml");
	}

	/// A renamed page's old url exports as a meta-refresh stub, so a static host
	/// keeps every link ever published resolving even though it serves no 301.
	#[beet_core::test]
	async fn exports_redirect_stubs() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			homepage: Some("https://beet.org".into()),
			..pkg_config!()
		});
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Route path="blog">
					<Redirect path="post-1" redirect="full-stack-bevy"/>
				</Route>
			})))
			.unwrap();
		world.flush();

		let out = BlobStore::temp();
		let out2 = out.clone();
		world
			.run_async_then(async move |world| {
				StaticExport::export(&world, root, &out2).await
			})
			.await
			.unwrap();
		out.get(&SmolPath::new("blog/post-1/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains(
				r#"<meta http-equiv="refresh" content="0; url=/blog/full-stack-bevy">"#,
			)
			.xpect_contains(
				r#"<link rel="canonical" href="https://beet.org/blog/full-stack-bevy">"#,
			)
			.xpect_contains(r#"<a href="/blog/full-stack-bevy">"#);
	}

	/// Exports `router` to a temp store, returning the written paths. The process
	/// stage decides the draft gate, so this is always the dev path; the prod
	/// path is asserted against [`StaticExport::exports`] directly.
	async fn export(world: &mut World, router: Entity) -> Vec<SmolPath> {
		let out = BlobStore::temp();
		world
			.run_async_then(async move |world| {
				StaticExport::export(&world, router, &out).await
			})
			.await
			.unwrap()
	}

	/// A router world for the draft-gate cases, with the `PackageConfig` the
	/// default `/app-info` route reads at render (see [`exports_static_scenes`]).
	fn draft_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world
	}

	/// Whether a route under `prefix` is in the exported set.
	fn exported(paths: &[SmolPath], prefix: &str) -> bool {
		paths.iter().any(|path| path.starts_with(prefix))
	}

	/// A `published` page plus a `secret` one eagerly marked
	/// `PageMeta { visibility: Draft }` (the codegen `BlobScene` shape).
	fn spawn_draft_router(world: &mut World) -> Entity {
		world
			.spawn((Router::with_defaults(), children![
				(
					render_action::fixed_func_route(
						"published",
						|| rsx! { <p>"Published"</p> }
					),
					HttpMethod::Get,
					PageRoute,
				),
				(
					render_action::fixed_func_route(
						"secret",
						|| rsx! { <p>"Secret"</p> }
					),
					HttpMethod::Get,
					PageRoute,
					PageMeta {
						visibility: PageVisibility::Draft,
						..default()
					},
				),
			]))
			.flush()
	}

	/// Non-prod builds export drafts so they can be previewed, end to end.
	#[beet_core::test]
	async fn dev_keeps_draft_routes() {
		let mut world = draft_world();
		let router = spawn_draft_router(&mut world);
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_true();
	}

	/// A prod stage drops the draft route, keeping every other qualifying one.
	#[beet_core::test]
	fn prod_drops_draft_routes() {
		let mut world = draft_world();
		let router = spawn_draft_router(&mut world);
		let paths = StaticExport::paths(&world, router, true).unwrap();
		exported(&paths, "published").xpect_true();
		exported(&paths, "secret").xpect_false();
	}

	/// Write a `published`/`secret` (frontmatter `visibility = "Draft"`) content dir under
	/// a per-test `name` (so parallel cases never share a directory) and return
	/// its root.
	// `RoutesDir` scans the filesystem store, so this is native-only.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	fn draft_content_dir(name: &str) -> AbsPathBuf {
		let root = fs_ext::workspace_root()
			.join("target/tests/export_static/drafts")
			.join(name);
		fs_ext::remove(&root).ok();
		fs_ext::write(root.join("published.md"), "# Published").unwrap();
		fs_ext::write(
			root.join("secret.md"),
			"+++\nvisibility = \"Draft\"\n+++\n\n# Secret",
		)
		.unwrap();
		AbsPathBuf::new(root).unwrap()
	}

	/// Spawn a `RoutesDir` router over `root`, settling the async runtime so the
	/// discovery scan (an async task) completes before the export walks the routes.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	async fn spawn_routes_dir(world: &mut World, root: AbsPathBuf) -> Entity {
		// compose the repo store on the router root so `RoutesDir` resolves it by
		// ancestry, then settle the discovery scan before the export walks the routes.
		let router = world
			.spawn((FsStore::new(root), Router::with_defaults(), children![
				RoutesDir::default()
			]))
			.flush();
		AsyncRunner::settle_async_tasks(world).await;
		router
	}

	/// The `RoutesDir` shape in dev: a scan-time `visibility = "Draft"` route is
	/// still exported for preview.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn dev_keeps_draft_routes_dir() {
		let mut world = draft_world();
		let router =
			spawn_routes_dir(&mut world, draft_content_dir("dev")).await;
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_true();
	}

	/// The `RoutesDir` shape in prod: scan-time frontmatter
	/// `visibility = "Draft"` excludes the discovered route from the export.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn prod_drops_draft_routes_dir() {
		let mut world = draft_world();
		let router =
			spawn_routes_dir(&mut world, draft_content_dir("prod")).await;
		let paths = StaticExport::paths(&world, router, true).unwrap();
		exported(&paths, "published").xpect_true();
		exported(&paths, "secret").xpect_false();
	}
}
