//! Static-site export driven by the runtime [`RouteTree`].
//!
//! Walks the router's route tree for every static-path scene route or route
//! marked [`ExportStrategy::Static`], renders each through the same dispatch path
//! a live request would take, and writes the resulting HTML to an output
//! [`BlobStore`] as `<path>/index.html`.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Renders every static route in the router to HTML.
///
/// A route is exported when its path is fully static, its method is `GET`, and
/// it is either a scene route or marked [`ExportStrategy::Static`]. Routes whose
/// [`ArticleMeta`] marks them a draft are skipped only on a `prod`
/// [`PackageConfig::stage`]; dev/staging builds export drafts so they can be
/// previewed.
pub async fn collect_static_html(
	world: &AsyncWorld,
	router: Entity,
) -> Result<Vec<(SmolPath, String)>> {
	let paths = world
		.with(move |world: &mut World| -> Result<Vec<SmolPath>> {
			let tree = world
				.entity(router)
				.get::<RouteTree>()
				.ok_or_else(|| {
					bevyhow!("router entity {router} has no RouteTree")
				})?
				.clone();
			// drafts are excluded only in production; the resource is present at
			// export, defaulting to dev (keep drafts) when unset.
			let is_prod = world
				.get_resource::<PackageConfig>()
				.is_some_and(|config| config.is_prod());

			let mut paths = Vec::new();
			for node in tree.flatten_nodes() {
				if !node.path.is_static() {
					continue;
				}
				if node
					.method
					.map(|method| method != HttpMethod::Get)
					.unwrap_or(false)
				{
					continue;
				}
				// a draft route is dropped only on a prod stage
				let is_draft = world
					.entity(node.entity)
					.get::<ArticleMeta>()
					.is_some_and(|meta| meta.draft);
				if is_prod && is_draft {
					continue;
				}
				let cache = world
					.entity(node.entity)
					.get::<ExportStrategy>()
					.copied()
					.unwrap_or_default();
				if node.is_scene() || cache == ExportStrategy::Static {
					paths.push(node.path.annotated_path());
				}
			}
			Ok(paths)
		})
		.await?;

	let entity = world.entity(router);
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

/// Renders every static route and writes it to the output store, returning the
/// written paths. A page writes to `<path>/index.html` (clean URLs); an asset
/// route with a file extension (eg `js/reactivity.js`) writes its raw file, so
/// the `<script src="/js/reactivity.js">` a reactive page references resolves and
/// the export is self-contained.
pub async fn export_static(
	world: &AsyncWorld,
	router: Entity,
	out: &BlobStore,
) -> Result<Vec<SmolPath>> {
	let pages = collect_static_html(world, router).await?;
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn exports_static_scenes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// `default_router` also wires the std-only `/app-info` scene route, which
		// reads a `PackageConfig` at render; insert one so it exports cleanly.
		world.insert_resource(pkg_config!());
		let router = world
			.spawn((default_router(), children![
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
				export_static(&world, router, &out2).await
			})
			.await
			.unwrap();

		// the two user scene routes plus the `app-info` scene and the
		// `js/reactivity.js` runtime asset, both wired by `default_router`.
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

	/// Exports `router` to a temp store, returning the written paths.
	async fn export(world: &mut World, router: Entity) -> Vec<SmolPath> {
		let out = BlobStore::temp();
		world
			.run_async_then(async move |world| {
				export_static(&world, router, &out).await
			})
			.await
			.unwrap()
	}

	/// A router world with the package `stage` set, so the draft gate keys off
	/// it deterministically rather than the build profile.
	fn world_with_stage(stage: &str) -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			stage: stage.into(),
			..pkg_config!()
		});
		world
	}

	/// Whether the export wrote a route under `prefix`.
	fn exported(written: &[SmolPath], prefix: &str) -> bool {
		written.iter().any(|path| path.starts_with(prefix))
	}

	/// A `published` route plus a `secret` route eagerly marked
	/// `ArticleMeta { draft: true }` (the codegen `BlobScene` shape).
	fn spawn_draft_router(world: &mut World) -> Entity {
		world
			.spawn((default_router(), children![
				(
					render_action::fixed_func_route(
						"published",
						|| rsx! { <p>"Published"</p> }
					),
					HttpMethod::Get,
				),
				(
					render_action::fixed_func_route(
						"secret",
						|| rsx! { <p>"Secret"</p> }
					),
					HttpMethod::Get,
					ArticleMeta {
						draft: true,
						..default()
					},
				),
			]))
			.flush()
	}

	/// Non-prod builds export drafts so they can be previewed.
	#[beet_core::test]
	async fn dev_keeps_draft_routes() {
		let mut world = world_with_stage("dev");
		let router = spawn_draft_router(&mut world);
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_true();
	}

	/// A `prod` stage drops the draft route from the export.
	#[beet_core::test]
	async fn prod_drops_draft_routes() {
		let mut world = world_with_stage("prod");
		let router = spawn_draft_router(&mut world);
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_false();
	}

	/// Write a `published`/`secret` (frontmatter `draft = true`) content dir under
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
			"+++\ndraft = true\n+++\n\n# Secret",
		)
		.unwrap();
		AbsPathBuf::new(root).unwrap()
	}

	/// Spawn a `RoutesDir` router over `root`, settling the async runtime so the
	/// discovery scan (an async task) completes before the export walks the routes.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	async fn spawn_routes_dir(world: &mut World, root: AbsPathBuf) -> Entity {
		// compose the site store on the router root so `RoutesDir` resolves it by
		// ancestry, then settle the discovery scan before the export walks the routes.
		let router = world
			.spawn((
				FsStore::new(root),
				default_router(),
				children![RoutesDir::default()],
			))
			.flush();
		AsyncRunner::settle_async_tasks(world).await;
		router
	}

	/// The `RoutesDir` shape in dev: a scan-time `draft = true` route is still
	/// exported for preview.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn dev_keeps_draft_routes_dir() {
		let mut world = world_with_stage("dev");
		let router =
			spawn_routes_dir(&mut world, draft_content_dir("dev")).await;
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_true();
	}

	/// The `RoutesDir` shape in prod: scan-time frontmatter `draft = true`
	/// excludes the discovered route from the export.
	#[cfg(all(feature = "markdown_parser", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn prod_drops_draft_routes_dir() {
		let mut world = world_with_stage("prod");
		let router =
			spawn_routes_dir(&mut world, draft_content_dir("prod")).await;
		let written = export(&mut world, router).await;
		exported(&written, "published").xpect_true();
		exported(&written, "secret").xpect_false();
	}
}
