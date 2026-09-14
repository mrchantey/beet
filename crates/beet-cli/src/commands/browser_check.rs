//! What the committed browser boot checks share: a page generated through the
//! real `<Wasm>` template, served with the workspace's artifacts and examples
//! on an ephemeral `HttpServer`, and a uniquely-ported headless chromium whose
//! console the check reads.

use beet::prelude::webdriver::*;
use beet::prelude::*;

/// A whole document around `body`, rendered through the router's templates so
/// the loader contract under test is the one a served page uses.
pub(crate) fn wasm_page(body: impl Bundle) -> Result<String> {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = world.spawn_template(Snippet::from_bundle(body))?.id();
	let body = HtmlRenderer::new()
		.render(&mut RenderContext::new(root, &mut world))?
		.to_string();
	format!(
		"<!doctype html>\n<html><head><meta charset=\"utf-8\"/></head><body>{body}</body></html>"
	)
	.xok()
}

/// Serve `page` at `/` plus the workspace's `assets/` and `examples/` on a
/// pre-bound port-0 `HttpServer` in a background app (its own thread,
/// mirroring `run_wasm_browser`'s server shape), returning the bound port.
/// The page is the root route, so a launch booting a `DomServer` lands where
/// the served entry's own page would; the server is not the process's
/// canonical one, so several checks serve at once in one test binary without
/// reading each other's port.
pub(crate) async fn serve_wasm_page(page: String) -> Result<u16> {
	let workspace = FsStore::new(AbsPath::new_workspace_rel("")?);
	let (mut server, on_spawn) =
		HttpServer::new_test(HttpServer::start_mini_with_tcp);
	server.canonical = false;
	let port = server.port.expect("a test server is pre-bound");
	std::thread::spawn(move || {
		let mut app = App::new();
		// RouterPlugin pulls ServerPlugin itself
		app.add_plugins((MinimalPlugins, AsyncPlugin, RouterPlugin));
		app.world_mut()
			.spawn((server, on_spawn, workspace, children![(
				Router::default(),
				children![
					route::new(
						"/",
						exchange_ext::handler(move |_| {
							Response::ok_body(page.clone(), MediaType::Html)
						})
					),
					// the artifact and the entry's repo, from the ancestor
					// workspace store
					AssetsDir {
						src: "assets".into(),
						prefix: default(),
						cache: default(),
					}
					.into_snippet_bundle(),
					AssetsDir {
						src: "examples".into(),
						prefix: default(),
						cache: default(),
					}
					.into_snippet_bundle(),
				],
			)]));
		app.run();
	});
	// the pre-bound listener accepts once its app thread is up
	poll_ext::poll_async_with(
		async || {
			std::net::TcpStream::connect(("127.0.0.1", port))
				.map(|_| ())
				.map_err(|err| bevyhow!("port {port} not listening: {err}"))
		},
		Duration::from_secs(30),
		poll_ext::DEFAULT_INTERVAL,
	)
	.await?;
	port.xok()
}

/// A uniquely-ported chromium driver, so a check never fights another suite's.
pub(crate) fn driver() -> Result<Client> {
	Client::default()
		.with_driver_port(HttpServer::free_port()?)
		.with_websocket_port(HttpServer::free_port()?)
		.xok()
}

/// Drain the console into `log`, streaming each entry for the person watching.
pub(crate) fn drain(console: &Collector<ConsoleEntry>, log: &mut String) {
	for entry in console.drain() {
		cross_log!("{}", entry.text);
		log.push_str(&entry.text);
		log.push('\n');
	}
}

/// Fail unless the artifact at `path` (workspace-relative) is built, naming
/// the recipe that builds it.
pub(crate) fn require_artifact(path: &str, recipe: &str) {
	if !AbsPath::new_workspace_rel(path)
		.and_then(fs_ext::exists)
		.unwrap_or_default()
	{
		panic!("missing artifact `{path}`, run `{recipe}`");
	}
}
