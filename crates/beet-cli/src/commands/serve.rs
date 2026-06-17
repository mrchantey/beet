use beet::prelude::*;

/// Request params for the [`Serve`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ServeParams {
	/// Servers to start, selected from those the site declares (eg `http`).
	/// Comma-separate to start several; absent starts every declared server.
	server: Option<String>,
	/// Watch the site dir: respawn its routes on change and live-reload browsers.
	watch: bool,
}

/// Serves a no-code BSX site: registers the site's `templates/` directory and
/// spawns its `main.bsx` entry as the app root, then triggers the server the
/// site declared and hands it the process, never returning. The site path is a
/// directory containing `main.bsx`, or the file itself:
///
/// ```sh
/// beet serve examples/bsx_site                 # start every server main.bsx declares
/// beet serve examples/bsx_site --server=http   # start only its http server
/// beet serve examples/bsx_site --server=http --watch  # serve with live reload
/// ```
///
/// `Serve` knows nothing about specific servers: it triggers a [`StartServer`]
/// built from the request (`--server=` selects which declared servers boot, the
/// rest of the params flow as boot config) on the site root, then yields forever.
/// Whatever the site declared owns the process from there: a [`CliServer`] runs
/// one exchange and exits, an [`HttpServer`] (which self-inserts [`KeepAlive`])
/// keeps it alive, and if the site declares no server, nothing runs.
#[action(route = "serve/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ServeParams>())]
pub async fn Serve(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let params = parts.params().parse_reflect::<ServeParams>()?;
	let request_params = parts.params().clone();
	let SiteEntry { site_dir, entry } = resolve_site(&site_arg(parts)?)?;
	let root = build_site(&cx.caller, site_dir.clone(), entry).await?;

	// run the render-diagnostics pass over the freshly built site and surface every
	// problem loudly in the console before serving (dev mode does not abort on an
	// error; the console output and the live-reload re-scan are the signal). A
	// `--watch` reload re-runs the pass via `reload_site`.
	check_routes(&cx.world(), root).await?.log();

	// the start built straight from the request: `--server=` selects which of the
	// site's declared servers boot, the rest of the params flow as boot config.
	let start =
		StartServer::from_request(root, params.server.as_deref(), request_params);
	let watch = params.watch;
	cx.caller
		.with_world(move |world, _caller| {
			world.entity_mut(root).trigger(move |_| start);
			// dev mode: watch the site dir, respawning routes and live-reloading
			// browsers via `<LiveReloadScript/>`.
			if watch {
				world.spawn(LiveReload::new(site_dir));
			}
		})
		.await?;
	// never return: the spawned server owns when the process exits.
	async_ext::yield_forever().await
}

/// Load the site to serve onto the caller's world, returning its root entity.
/// Shared by [`Serve`] and the `export-static` command; the site declares its
/// own server and app routes in `main.bsx`, so this only loads it.
pub(crate) async fn build_site(
	caller: &AsyncEntity,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	caller
		.with_world(move |world, _caller| load_site(world, site_dir, entry))
		.await?
}

/// The synchronous site load: register the site's `templates/`, set the
/// [`SiteRoot`] (which `<RoutesDir/>` resolves against), and spawn the
/// `main.bsx` entry as the app root.
pub(crate) fn load_site(
	world: &mut World,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	let templates = site_dir.join("templates");
	if fs_ext::exists(&templates)? {
		world.register_bsx_templates(templates)?;
	}
	world.insert_resource(SiteRoot(site_dir));
	BsxTemplate::load_entry(world, &entry)?.spawn(world)
}

/// The `*site` path argument, joined back into a path. Errors with usage if empty.
pub(crate) fn site_arg(parts: &RequestParts) -> Result<String> {
	let site = parts
		.get_params("site")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if site.is_empty() {
		bevybail!("usage: beet serve <site-dir>");
	}
	site.xok()
}

/// A resolved site: its root directory and the `main.bsx` entry file.
pub(crate) struct SiteEntry {
	pub site_dir: AbsPathBuf,
	pub entry: AbsPathBuf,
}

/// Resolve a site path: a directory containing `main.bsx`, or the file itself.
/// Relative paths resolve against the cwd; an absolute positional round-trips as
/// absolute (the `*site` capture keeps its leading `/`), so any cwd resolves it.
pub(crate) fn resolve_site(site: &str) -> Result<SiteEntry> {
	let path = AbsPathBuf::new(site)?;
	if !fs_ext::exists(&path)? {
		bevybail!("site not found: {site}");
	}
	if path.is_dir() {
		let entry = path.join("main.bsx");
		if !fs_ext::exists(&entry)? {
			bevybail!("no main.bsx in {path}");
		}
		SiteEntry {
			site_dir: path,
			entry,
		}
		.xok()
	} else {
		let site_dir = path
			.parent()
			.ok_or_else(|| bevyhow!("site file has no parent dir: {path}"))?;
		SiteEntry {
			site_dir,
			entry: path,
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn site_path() -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel("examples/bsx_site").unwrap()
	}

	#[beet::test]
	fn resolves_dir_and_entry_file() {
		let dir = resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		dir.entry.xpect_eq(site_path().join("main.bsx"));
		let file = resolve_site(
			site_path().join("main.bsx").to_string_lossy().as_ref(),
		)
		.unwrap();
		file.site_dir.xpect_eq(site_path());
		resolve_site("not/a/site")
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("site not found");
	}

	/// The site declares its own server and app routes: loading `main.bsx` yields
	/// a root carrying the markup-declared `HttpServer` plus the default app
	/// routes it requested with `<DefaultAppRoutes/>` (eg `/js/reactivity.js`), so
	/// `Serve` only has to trigger the start.
	#[beet::test]
	fn site_declares_server_and_app_routes() {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		)
			.into_world();
		let SiteEntry { site_dir, entry } =
			resolve_site(site_path().to_string_lossy().as_ref()).unwrap();
		let root = load_site(&mut world, site_dir, entry).unwrap();
		world.flush();
		// the markup `<Router {(.., HttpServer{..})}>` declared a server
		world.entity(root).contains::<HttpServer>().xpect_true();
		// and `<DefaultAppRoutes/>` wired the reactivity-runtime route
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["js", "reactivity.js"])
			.xpect_some();
	}
}
