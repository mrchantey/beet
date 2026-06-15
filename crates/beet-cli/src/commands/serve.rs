use beet::prelude::*;

/// Request params for the [`Serve`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ServeParams {
	/// Server to start: absent renders the route once, `http` serves the site
	/// over HTTP. Comma-separate to start several.
	server: Option<String>,
	/// The route to render when no long-running server is started, defaults to
	/// the home route.
	route: Option<String>,
	/// When serving over HTTP, watch the site dir: respawn its routes on change
	/// and live-reload connected browsers.
	watch: bool,
}

/// Serves a no-code BSX site: registers the site's `templates/` directory,
/// spawns its `main.bsx` entry as the app root, then either renders one route or
/// starts a long-running server selected by `--server`. The site path is a
/// directory containing `main.bsx`, or the file itself:
///
/// ```sh
/// beet serve examples/bsx_site                        # render the home route
/// beet serve examples/bsx_site --route=docs/routing   # render a named route
/// beet serve examples/bsx_site --server=http          # serve the site over http
/// beet serve examples/bsx_site --server=http --watch  # serve with live reload
/// ```
///
/// A long-running `--server` (eg `http`) is started by triggering a
/// [`StartServer`] built agnostically from the request on the site root, the way
/// any host boots its servers. With no long-running server selected, `Serve`
/// renders the requested route once and returns it, the way `beet`'s own
/// entrypoint serves a single request.
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

	// a long-running server (`http`) is started on the site root and keeps the
	// process alive; absent one, render the requested route once and return it.
	if params.server.as_deref().is_some_and(is_long_running) {
		// the start built straight from the request: the `--server=` value selects
		// the servers, the rest of the params flow through as boot config.
		let start = StartServer::from_request(
			root,
			params.server.as_deref(),
			request_params,
		);
		let watch = params.watch;
		let serving =
			format!("serving {site_dir} with {}\n", params.server.unwrap());
		cx.caller
			.with_world(move |world, _caller| {
				// spawn the long-running server, then trigger its start.
				world.entity_mut(root).insert(HttpServer::default());
				world.entity_mut(root).trigger(move |_| start);
				// dev mode: watch the site dir, respawning routes and live-reloading
				// browsers via `<LiveReloadScript/>`.
				if watch {
					world.spawn(LiveReload::new(site_dir));
				}
			})
			.await?;
		Response::ok_text(serving).xok()
	} else {
		render_route(cx, root, params.route).await
	}
}

/// Whether a `--server` value names a long-running server (one that keeps the
/// process alive), eg `http`. A bare/absent value renders one route and returns.
fn is_long_running(server: &str) -> bool {
	server
		.split(',')
		.map(str::trim)
		.any(|name| matches!(name, "http" | "tui"))
}

/// Build the site world on the caller: register the site templates, spawn the
/// `main.bsx` entry as the root, and layer the default app routes onto it.
/// Returns the spawned site root entity. Shared by [`Serve`] and the
/// `export-static` command.
pub(crate) async fn build_site(
	caller: &AsyncEntity,
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
) -> Result<Entity> {
	caller
		.with_world(move |world, _caller| -> Result<Entity> {
			let templates = site_dir.join("templates");
			if fs_ext::exists(&templates)? {
				world.register_bsx_templates(templates)?;
			}
			// `<RoutesDir src=".."/>` resolves against the site root
			world.insert_resource(SiteRoot(site_dir));
			let root = BsxTemplate::load_entry(world, &entry)?.spawn(world)?;
			// the default app routes `default_router` wires for codegen hosts,
			// including the cached `/js/reactivity.js` runtime the reactive
			// renderer's auto-injected script loads
			world.spawn((ChildOf(root), app_info()));
			world.spawn((ChildOf(root), reactivity_js_route()));
			world.spawn((ChildOf(root), analytics_handler()));
			root.xok()
		})
		.await?
}

/// Render the requested route through the site router, forwarding the original
/// request so eg `--accept` content negotiation holds. The same one-exchange
/// shape as `beet`'s own CLI entrypoint.
async fn render_route(
	cx: ActionContext<Request>,
	root: Entity,
	route: Option<String>,
) -> Result<Response> {
	let route = route
		.unwrap_or_default()
		.split('/')
		.filter(|segment| !segment.is_empty())
		.map(SmolStr::from)
		.collect();
	let (mut parts, body) = cx.input.into_parts();
	*parts.url_mut() = parts.url().clone().with_path(route);
	cx.caller
		.world()
		.entity(root)
		.exchange(Request::from_parts(parts, body))
		.await
		.xok()
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
/// Relative paths resolve against the cwd; route segments lose a leading `/`,
/// so the absolute form is retried before erroring.
pub(crate) fn resolve_site(site: &str) -> Result<SiteEntry> {
	let path = AbsPathBuf::new(site)?;
	let path = match fs_ext::exists(&path)? {
		true => path,
		false => AbsPathBuf::new(format!("/{site}"))?,
	};
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

	/// Render `req` through a host carrying only the [`Serve`] route. The style
	/// plugin mirrors `beet`'s own entrypoint, so the site's `<Stylesheet/>`
	/// resolves its rules as it does in production.
	async fn run(req: Request) -> String {
		let mut world = (
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin::default(),
		)
			.into_world();
		let host = world.spawn((Router, children![Serve])).id();
		world
			.entity_mut(host)
			.call::<Request, Response>(
				req.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
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

	#[beet::test]
	fn long_running_classifies_servers() {
		is_long_running("http").xpect_true();
		is_long_running("tui").xpect_true();
		is_long_running("http, tui").xpect_true();
		is_long_running("cli").xpect_false();
		is_long_running("").xpect_false();
	}

	#[beet::test]
	async fn renders_home_route_in_cli_mode() {
		run(Request::get(format!("serve/{}", site_path())))
			.await
			.as_str()
			// the entry's layout document wraps the rendered route
			.xpect_contains("<html lang=\"en\">")
			// the markup `<PackageConfig/>` reached the layout's `@res` binding
			.xpect_contains("BSX Site");
	}

	/// The host layers the default app routes onto the markup-declared router,
	/// backed by the `<PackageConfig/>` declared in `main.bsx`.
	#[beet::test]
	async fn wires_default_app_routes() {
		run(Request::get(format!("serve/{}?route=app-info", site_path())))
			.await
			.as_str()
			.xpect_contains("App Info")
			.xpect_contains("BSX Site");
	}

	#[beet::test]
	async fn renders_named_route() {
		run(Request::get(format!(
			"serve/{}?route=docs/routing",
			site_path()
		)))
		.await
		.as_str()
		.xpect_contains("<html lang=\"en\">")
		.xpect_contains("Routing");
	}

	/// `--server=http` starts the long-running HTTP server on the site root
	/// (returning a serving message) rather than rendering a single route.
	#[beet::test]
	async fn starts_http_server() {
		// no `server` backend feature in this build, so install the runtime hook
		// the HTTP start observer invokes (idempotent across cases).
		set_http_server(|_| Box::pin(async { Ok(()) })).ok();
		run(Request::get(format!("serve/{}?server=http", site_path())))
			.await
			.as_str()
			.xpect_contains("serving")
			.xpect_contains("http");
	}
}
