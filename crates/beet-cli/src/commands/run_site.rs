use beet::prelude::*;

/// Request params for the [`Run`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct RunParams {
	/// Server backend: `cli` renders a route once, `http` serves the site.
	server: Option<String>,
	/// The route to render in `cli` mode, defaults to the home route.
	route: Option<String>,
	/// In `http` mode, watch the site dir: respawn its routes on change and
	/// live-reload connected browsers.
	watch: bool,
}

/// Runs a no-code BSX site: registers the site's `templates/` directory,
/// spawns its `main.bsx` entry as the app root, then serves it via the
/// selected backend. The site path is a directory containing `main.bsx`, or
/// the file itself:
///
/// ```sh
/// beet run examples/bsx_site                        # render the home route
/// beet run examples/bsx_site --route=docs/routing   # render a named route
/// beet run examples/bsx_site --server=http          # serve the site over http
/// beet run examples/bsx_site --server=http --watch  # serve with live reload
/// ```
#[action(route = "run/*site", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RunParams>())]
pub async fn Run(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx
		.input
		.request_parts()
		.params()
		.parse_reflect::<RunParams>()?;
	let server = ServerKind::parse(&params)?;
	let site = cx
		.input
		.request_parts()
		.get_params("site")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if site.is_empty() {
		bevybail!("usage: beet run <site-dir>");
	}
	let SiteEntry { site_dir, entry } = resolve_site(&site)?;
	let serving = format!("serving {site_dir} over http\n");
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
		ServerKind::Http => {
			let watch = params.watch;
			cx.caller
				.with_world(move |world, _caller| {
					world.entity_mut(root).insert(HttpServer::default());
					// dev mode: watch the site dir, respawning routes and
					// live-reloading browsers via `<LiveReloadScript/>`
					if watch {
						world.spawn(LiveReload::new(site_dir));
					}
					// keep the host alive after this command's response streams
					world.insert_resource(KeepAlive);
				})
				.await?;
			Response::ok_text(serving).xok()
		}
		// render the requested route through the site router, forwarding the
		// original request so eg `--accept` content negotiation holds
		ServerKind::Cli => {
			let route = params
				.route
				.unwrap_or_default()
				.split('/')
				.filter(|segment| !segment.is_empty())
				.map(String::from)
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

/// The server backend selected by `--server`, defaulting to `cli`.
enum ServerKind {
	Cli,
	Http,
}

impl ServerKind {
	fn parse(params: &RunParams) -> Result<Self> {
		match params
			.server
			.as_deref()
			.map(|server| server.to_lowercase())
			.as_deref()
		{
			None | Some("cli") => Self::Cli.xok(),
			Some("http") => Self::Http.xok(),
			Some(other) => {
				bevybail!("invalid --server '{other}', expected 'cli' or 'http'")
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

	/// Render `req` through a host carrying only the [`Run`] route.
	async fn run(req: Request) -> String {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let host = world.spawn((Router, children![Run])).id();
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
		run(Request::get(format!("run/{}", site_path())))
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
			"run/{}?route=app-info",
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
			"run/{}?route=docs/routing",
			site_path()
		)))
		.await
		.as_str()
		.xpect_contains("<html lang=\"en\">")
		.xpect_contains("Routing");
	}
}
