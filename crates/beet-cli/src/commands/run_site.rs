use beet::prelude::*;

/// Request params for the [`Serve`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct ServeParams {
	/// Server backend: `cli` renders a route once, `http` serves the site,
	/// `export` statically exports every static route to `<site>/dist`.
	server: Option<String>,
	/// The route to render in `cli` mode, defaults to the home route.
	route: Option<String>,
	/// In `http` mode, watch the site dir: respawn its routes on change and
	/// live-reload connected browsers.
	watch: bool,
}

/// Serves a no-code BSX site: registers the site's `templates/` directory,
/// spawns its `main.bsx` entry as the app root, then serves it via the
/// selected backend. The site path is a directory containing `main.bsx`, or
/// the file itself:
///
/// ```sh
/// beet serve examples/bsx_site                        # render the home route
/// beet serve examples/bsx_site --route=docs/routing   # render a named route
/// beet serve examples/bsx_site --server=http          # serve the site over http
/// beet serve examples/bsx_site --server=http --watch  # serve with live reload
/// beet serve examples/bsx_site --server=export        # static export to dist
/// ```
///
/// The `http` backend is spawned onto the site root as a [`ServerBackend`]
/// component: its `#[require(Server)]` pulls in the [`Server`] orchestrator,
/// which starts it and inserts [`KeepAlive`]. `cli` short-circuits to a single
/// rendered exchange, the way `beet`'s own entrypoint serves one request.
/// `export` renders every static route once and writes it under `<site>/dist`.
#[action(route = "serve/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ServeParams>())]
pub async fn Serve(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx
		.input
		.request_parts()
		.params()
		.parse_reflect::<ServeParams>()?;
	let server = ServeKind::parse(&params)?;
	let site = cx
		.input
		.request_parts()
		.get_params("site")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if site.is_empty() {
		bevybail!("usage: beet serve <site-dir>");
	}
	let SiteEntry { site_dir, entry } = resolve_site(&site)?;
	// build the site world: templates, site root, entry
	let build_dir = site_dir.clone();
	let root = cx
		.caller
		.with_world(move |world, _caller| -> Result<Entity> {
			let templates = build_dir.join("templates");
			if fs_ext::exists(&templates)? {
				world.register_bsx_templates(templates)?;
			}
			// `<RoutesDir src=".."/>` resolves against the site root
			world.insert_resource(SiteRoot(build_dir));
			let root = BsxTemplate::load_entry(world, &entry)?.spawn(world)?;
			// the default app routes `default_router` wires for codegen hosts
			world.spawn((ChildOf(root), app_info()));
			world.spawn((ChildOf(root), analytics_handler()));
			root.xok()
		})
		.await??;
	match server {
		ServeKind::Http => {
			let watch = params.watch;
			let serving = format!("serving {site_dir} over http\n");
			cx.caller
				.with_world(move |world, _caller| {
					// spawning the `HttpServer` backend pulls in the `Server`
					// orchestrator (`#[require(Server)]`), which starts the listener.
					world.entity_mut(root).insert(HttpServer::default());
					// keep this entrypoint's process alive after its response
					// streams: set `KeepAlive` synchronously here (rather than rely
					// on the orchestrator's async insert) so the forwarding
					// `CliServer` sees it before deciding whether to exit.
					world.insert_resource(KeepAlive);
					// dev mode: watch the site dir, respawning routes and
					// live-reloading browsers via `<LiveReloadScript/>`
					if watch {
						world.spawn(LiveReload::new(site_dir));
					}
				})
				.await?;
			Response::ok_text(serving).xok()
		}
		// statically export every static route to `<site>/dist`, the no-main.rs
		// replacement for the example's old `export` route. The site root is the
		// router (the entry's root element is built into it), so it exports directly.
		ServeKind::Export => {
			let out = BlobStore::new(FsStore::new(site_dir.join("dist")));
			let written = export_static(&cx.world(), root, &out).await?;
			Response::ok_text(format!(
				"exported {} routes to dist\n",
				written.len()
			))
			.xok()
		}
		// render the requested route through the site router, forwarding the
		// original request so eg `--accept` content negotiation holds. This is
		// the same one-exchange shape as `beet`'s own CLI entrypoint.
		ServeKind::Cli => {
			let route = params
				.route
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
	}
}

/// The site backend selected by `--server`, defaulting to `cli`. Serving a site
/// is a one-shot render (`cli`), a long-running [`HttpServer`], or a one-shot
/// static `export` of every static route.
enum ServeKind {
	Cli,
	Http,
	Export,
}

impl ServeKind {
	fn parse(params: &ServeParams) -> Result<Self> {
		match params
			.server
			.as_deref()
			.map(|server| server.to_lowercase())
			.as_deref()
		{
			None | Some("cli") => Self::Cli.xok(),
			Some("http") => Self::Http.xok(),
			Some("export") => Self::Export.xok(),
			Some(other) => {
				bevybail!(
					"invalid --server '{other}', expected 'cli', 'http' or 'export'"
				)
			}
		}
	}
}

/// A resolved site: its root directory and the `main.bsx` entry file.
struct SiteEntry {
	site_dir: AbsPathBuf,
	entry: AbsPathBuf,
}

/// Resolve a site path: a directory containing `main.bsx`, or the file itself.
/// Relative paths resolve against the cwd; route segments lose a leading `/`,
/// so the absolute form is retried before erroring.
fn resolve_site(site: &str) -> Result<SiteEntry> {
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
		run(Request::get(format!(
			"serve/{}?route=app-info",
			site_path()
		)))
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

	/// `--server=export` statically exports the markup site to `<site>/dist`,
	/// the no-main.rs replacement for the example's old `export` route.
	#[beet::test]
	async fn exports_static_site() {
		run(Request::get(format!("serve/{}?server=export", site_path())))
			.await
			.as_str()
			.xpect_contains("exported")
			.xpect_contains("routes to dist");
		// the home route landed as the dist index, wrapped in the layout
		fs_ext::read_to_string(site_path().join("dist/index.html"))
			.unwrap()
			.as_str()
			.xpect_contains("<html lang=\"en\">")
			.xpect_contains("BSX Site");
	}
}
