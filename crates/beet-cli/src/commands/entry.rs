//! Resolving and loading a no-code entry, shared by the [`Check`],
//! [`ExportStatic`] and [`ExportPdf`] commands.
//!
//! An entry is a directory containing an entry document ([`entry_build::ENTRY_NAMES`], eg
//! `main.bsx`) or the entry file itself, loaded as the app root. The entry declares its own servers and app routes;
//! these helpers only resolve and load it.

use beet::prelude::*;

/// The repo store selector every entry-loading command shares (`check`,
/// `export-static`, `export-pdf`), naming the backend the entry loads through.
///
/// A command loads an entry into *this* process rather than launching one, so
/// it takes its own `--entry-repo` (documented in its `--help`) rather than the
/// launch's `--repo`, which the launch consumes as its own repo store and
/// never hands a route.
#[derive(Reflect)]
pub(crate) struct EntryParams {
	/// The repo store the entry loads through, eg
	/// `--entry-repo=s3://my-bucket`. Defaults to the filesystem, rooted at the
	/// entry directory.
	entry_repo: Option<StoreUri>,
}

impl EntryParams {
	/// Load the entry at `entry_path` onto the caller's world through its
	/// [`BlobStore`], returning its root entity. Resolution is the binary's own
	/// [`entry_build::resolve_entry_path`] (`--entry-repo` picks the backend,
	/// the entry's `<RepoRoot src>` rebases the root), so the same resolution
	/// serves `beet --entry=..` and these commands.
	///
	/// The commands render the entry rather than serve it, so the build
	/// disarms the root ([`DisableCallOnReady`]) and the entry's `CallOnReady`
	/// verb stays dormant. The reads go through the store
	/// ([`entry_build::read_sources`]/[`entry_build::build_root`]), the same
	/// agnostic core the native binary and the wasm Worker use, so the command
	/// is store-driven rather than filesystem-bound.
	///
	/// The wait on the entry's discovery tasks is bounded by
	/// [`ONE_SHOT_SETTLE_DEADLINE`]: a one-shot command produces a result and
	/// exits, so a dependency that never resolves must fail it rather than
	/// hang it.
	pub async fn load(
		&self,
		caller: &AsyncEntity,
		entry_path: &str,
	) -> Result<Entity> {
		let prescans = caller
			.with_world(|world, _| {
				world.get_resource_or_init::<PrescanRegistry>().clone()
			})
			.await?;
		let ResolvedEntry {
			repo_store,
			entry_name,
			prescan,
			..
		} = entry_build::resolve_entry_path(
			&prescans,
			self.entry_repo.as_ref(),
			None,
			entry_path,
		)
		.await?;
		let sources =
			entry_build::read_sources(&repo_store, entry_name, prescan).await?;
		let root = caller
			.with_world(move |world, _| {
				entry_build::build_root(
					world,
					repo_store,
					sources,
					DisableCallOnReady,
				)
			})
			.await??;
		// the entry's `<RoutesDir/>` discovery runs as an async task; wait for it
		// so every discovered route exists before the caller renders/exports the
		// entry.
		TemplatePending::settle_before(
			caller.world(),
			ONE_SHOT_SETTLE_DEADLINE,
		)
		.await?;
		Ok(root)
	}
}

/// How long a one-shot command (`check`, `export-static`, `export-pdf`) waits on
/// the entry's dependencies before failing.
///
/// Generous, since it covers a whole `<RoutesDir>` scan over a large site on a
/// cold cache; it exists to turn a genuinely stuck
/// dependency into a named error rather than a hung command, not to police build
/// speed.
const ONE_SHOT_SETTLE_DEADLINE: Duration = Duration::from_secs(60);

/// The entry's own directory (an entry file's parent, or the dir itself), the
/// default home for command outputs like `dist/` and `site.pdf`. Deliberately
/// not the `<RepoRoot>`-widened root, so outputs land beside the entry.
pub(crate) fn entry_dir(entry_path: &str) -> Result<AbsPath> {
	let path = AbsPath::new(entry_path)?;
	if path.extension().is_some() {
		path.parent()
			.ok_or_else(|| bevyhow!("entry `{path}` has no parent directory"))
	} else {
		path.xok()
	}
}

/// The `*entry` path argument, joined back into a path. Errors with usage if empty.
pub(crate) fn entry_arg(parts: &RequestParts) -> Result<String> {
	let entry = parts
		.get_params("entry")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if entry.is_empty() {
		return Err(HttpError::bad_request(
			"usage: beet <command> <entry-dir>",
		)
		.into());
	}
	entry.xok()
}

/// Wait for the process [`CanonicalPort`] to register, ie the freshly booted
/// canonical http server's bound port. Shared by the commands that boot a
/// server on an ephemeral port and then drive it (`export-pdf`, the
/// browser-hosted `run-wasm`).
pub(crate) async fn wait_for_port() -> Result<u16> {
	poll_ext::poll_result_with(
		CanonicalPort::get,
		Duration::from_secs(5),
		Duration::from_millis(25),
	)
	.await
}

/// A world able to load and render a no-code entry for the `check` /
/// `export-static` / entry-load tests. It mirrors the running app's widget surface:
/// [`BsxDefaultsPlugin`] registers the beet_ui widget templates (`<Button>`/`<Form>`/…)
/// and the default `bx:` event vocabulary, so a markup entry using live widgets renders
/// here as it would under `BeetPlugins`. It is added *before* [`RouterPlugin`] so its
/// inner `BsxPlugin` registers once (the router's charcell stack reaches it through the
/// idempotent `install_types`). [`RouterPlugin`] registers the markup-declarable
/// server *types* (`TuiServer`, `SshTuiServer`) by feature, so an entry's server
/// spread resolves without the ssh runtime systems (which need an input backend);
/// the disarmed build keeps the declared servers dormant anyway.
#[cfg(test)]
pub(crate) fn render_world() -> World {
	(
		AsyncPlugin,
		BsxDefaultsPlugin,
		RouterPlugin,
		material::MaterialStylePlugin::default(),
	)
		.into_world()
}

/// A small clean site in a temp dir: a router over its `routes/`, wrapped
/// in a layout from its `templates/`, and a page linking another. Written
/// rather than borrowed from `examples/`, whose sites declare derived
/// files a fresh checkout has not generated, for the `check` and
/// `export-static` tests.
#[cfg(test)]
pub(crate) fn clean_site() -> TempDir {
	let dir = TempDir::new().unwrap();
	for (path, content) in [
		(
			"main.bsx",
			r#"<Router {Layout(Document("templates/Layout.bsx"))}>
	<PackageConfig title="Clean Site"/>
	<RoutesDir src="routes"/>
</Router>"#,
		),
		(
			"templates/Layout.bsx",
			"<html lang=\"en\"><RouteHead/><body><main><slot/></main></body></html>",
		),
		(
			"routes/index.md",
			"+++\ntitle = \"Home\"\n+++\n\n# Home\n\nRead [the about page](/about).",
		),
		("routes/about.md", "+++\ntitle = \"About\"\n+++\n\n# About"),
	] {
		fs_ext::write(dir.path().join(path), content).unwrap();
	}
	dir
}

#[cfg(test)]
mod test {
	use super::*;

	fn entry_path() -> AbsPath {
		AbsPath::new_workspace_rel("examples/beetmania").unwrap()
	}

	/// The prescan set the render world registers.
	fn prescans(world: &World) -> PrescanRegistry {
		world.resource::<PrescanRegistry>().clone()
	}

	/// The shared [`entry_build::resolve_entry_path`] serves the commands' positional: a dir resolves
	/// to its highest-priority [`entry_build::ENTRY_NAMES`] entry through the store, an entry
	/// file names itself, and a dir with no entry document errors with guidance.
	#[beet::test]
	async fn resolves_dir_and_entry_file() {
		let prescans = prescans(&render_world());
		// a dir resolves to its highest-priority `entry_build::ENTRY_NAMES` entry (`main.bsx` here)
		let dir = entry_build::resolve_entry_path(
			&prescans,
			None,
			None,
			entry_path().as_str(),
		)
		.await
		.unwrap();
		dir.entry_name.xpect_eq("main.bsx");
		// passing the entry file itself roots the store at its parent
		let file = entry_build::resolve_entry_path(
			&prescans,
			None,
			None,
			entry_path().join("main.bsx").as_str(),
		)
		.await
		.unwrap();
		file.entry_name.xpect_eq("main.bsx");
		file.watch_dir.xpect_eq(Some(entry_path()));
		// a non-`main.bsx` entry name is still discovered (the search spans entry_build::ENTRY_NAMES)
		let tmp = TempDir::new().unwrap();
		fs_ext::write(tmp.path().join("main.json"), "{}").unwrap();
		entry_build::resolve_entry_path(
			&prescans,
			None,
			None,
			tmp.path().as_str(),
		)
		.await
		.unwrap()
		.entry_name
		.xpect_eq("main.json");
		// a dir with no entry document errors with guidance
		let empty = TempDir::new().unwrap();
		entry_build::resolve_entry_path(
			&prescans,
			None,
			None,
			empty.path().as_str(),
		)
		.await
		.err()
		.unwrap()
		.to_string()
		.xpect_contains("no entry document");
	}

	/// The example entry built through its resolved store, its `<RoutesDir>`
	/// scans settled, answering the root and the site's router.
	async fn build_example(world: &mut World) -> (Entity, Entity) {
		let ResolvedEntry {
			repo_store,
			entry_name,
			prescan,
			..
		} = entry_build::resolve_entry_path(
			&prescans(world),
			None,
			None,
			entry_path().as_str(),
		)
		.await
		.unwrap();
		let sources =
			entry_build::read_sources(&repo_store, entry_name, prescan)
				.await
				.unwrap();
		let root = entry_build::build_root(
			world,
			repo_store,
			sources,
			DisableCallOnReady,
		)
		.unwrap();
		// the `<RoutesDir>` scan settles, so the site's router serves pages
		AsyncRunner::settle_async_tasks(world).await;
		let router = world
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				find_router,
				root,
			)
			.unwrap()
			.unwrap();
		(root, router)
	}

	/// The entry declares its own servers and app routes: loading its entry document
	/// through the resolved store yields a command dispatcher root whose `serve`
	/// route carries a markup-declared `<HttpServer>` over the site's router,
	/// plus the default app routes it requested with `<DefaultAppRoutes/>`
	/// (eg `/app-info`).
	#[beet::test]
	async fn entry_declares_server_and_app_routes() {
		let mut world = render_world();
		let (root, router) = build_example(&mut world).await;
		// the root dispatches commands, and the markup `<HttpServer>` owns the
		// `serve` route's boot, with the site's router as its child
		world.entity(root).contains::<CliServer>().xpect_true();
		let serve = world.entity(router).get::<ChildOf>().unwrap().parent();
		world.entity(serve).contains::<HttpServer>().xpect_true();
		// and `<DefaultAppRoutes/>` wired the app-info route into the site's
		// own url space
		RouteTree::of(&world, root)
			.unwrap()
			.find(&["app-info"])
			.xpect_some();
	}

	/// The deployed example sits behind an edge that caches what the origin's
	/// `Cache-Control` allows, and a sync purges it: a page carries the
	/// edge-cacheable html policy, never the edge's own default, which a
	/// purge-free sync once left serving the old pages for hours.
	#[beet::test]
	async fn pages_carry_the_edge_cache_policy() {
		let mut world = render_world();
		let (_, router) = build_example(&mut world).await;
		world
			.entity_mut(router)
			.exchange(
				Request::get("/")
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.headers
			.get::<header::CacheControl>()
			.unwrap()
			.unwrap()
			.xpect_contains("s-maxage=");
	}
}
