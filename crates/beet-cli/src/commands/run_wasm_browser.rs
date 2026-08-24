//! The browser host for the wasm runner, selected by `--wasm-host=browser` /
//! `BEET_WASM_HOST=browser` ([`WasmHost::Browser`]): where the deno
//! host gives a wasm module a js runtime with fs access and no dom, this
//! serves the `wasm-bindgen` output through the standard beet server stack,
//! boots a headless chromium through the in-house webdriver, streams the page
//! console to stdout, and exits with the module's verdict.
//!
//! The page itself is the standard browser embedding, nothing test-specific:
//! `index.html` loads `browser.js` (the browser twin of `deno.ts`), which
//! installs the `js_runtime` globals and awaits the module's exported
//! `test_start` / `start` per the shared host contract. Runner args and the
//! [`BootstrapConfig`] ride the url query, which the wasm side already parses
//! exactly like argv (`search_params_ext::location_args`); the one runner-fed
//! side channel is `env.json` (`WORKSPACE_ROOT`). No fs shims are installed,
//! so snapshot and store tests stay deno and native territory.

use beet::prelude::webdriver::*;
use beet::prelude::*;
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

const INDEX_HTML: &str = include_str!("index.html");
const BROWSER_JS: &str = include_str!("browser.js");

/// How long the whole module may run before the driver gives up. Generous:
/// the runner's per-test timeouts are the real watchdog, this only catches a
/// module that never reports at all.
const SUITE_DEADLINE: Duration = Duration::from_secs(600);

/// Serve `runner_dir`, drive a headless browser at it, stream the console and
/// propagate the module's exit code.
pub(crate) async fn run(
	cx: &ActionContext<Request>,
	runner_dir: &Path,
	bootstrap: &BootstrapConfig,
	args: Vec<String>,
	chrome_args: Vec<String>,
) -> Result {
	// the host page + the runner-provided env
	fs_ext::write_async(runner_dir.join("index.html"), INDEX_HTML).await?;
	fs_ext::write_async(runner_dir.join("browser.js"), BROWSER_JS).await?;
	fs_ext::write_async(runner_dir.join("env.json"), env_json()).await?;

	// serve the runner dir through the standard stack: an ephemeral
	// `HttpServer`, an `FsStore` and a root `ServeBlobs` mount, whose
	// extensionless rule resolves `/` to `index.html`
	let store = FsStore::new(AbsPathBuf::new(runner_dir.to_path_buf())?);
	let root = cx
		.world()
		.with(move |world: &mut World| {
			// the dispatch host is the server's *child* (one action per
			// entity: the server owns the boot slot, the router the dispatch)
			world
				.spawn((
					HttpServer {
						port: Some(0),
						..default()
					},
					store,
					children![(
						// bare: the default routes (app-info, help) sit at
						// the root level and conflict with the greedy mount
						Router::default(),
						children![
							ServeBlobs {
								prefix: default(),
								cache: default(),
							}
							.into_snippet_bundle()
						],
					)],
				))
				.id()
		})
		.await;
	// boot parks on the server's `Running`, so launch fire-and-forget and
	// wait for the bound port, mirroring `export-pdf`
	cx.world()
		.run_async_local(move |world| async move {
			CallOnReady::call_recursive(world.entity(root), || {
				Request::from_cli_str("--server=http")
			})
			.await?;
			Ok(())
		})
		.await;
	let port = super::wait_for_port().await?;

	// runner args and the bootstrap ride the query, parsed by the wasm side
	// exactly like argv
	let mut query_args = args;
	query_args.extend(
		bootstrap
			.to_argv()?
			.into_iter()
			.map(|token| token.to_string()),
	);
	let url = format!("http://127.0.0.1:{port}/{}", args_to_query(&query_args));

	// drive: the console streams live, the verdict lands on `__beet_exit`.
	// `--chrome-args` rides the request (see `RunWasm`), passing extra browser
	// flags through to the session, eg `--enable-unsafe-webgpu --use-angle=gl`
	// (which also lifts the default `--disable-gpu`) so a dom test can exercise
	// a WebGPU-granting chrome.
	let session_opts = NewSessionOptions::default()
		.with_disable_gpu(chrome_args.is_empty())
		.with_extra_args(chrome_args);
	let mut browser = Browser::new_with_opts(
		Client::default()
			.with_driver_port(free_port()?)
			.with_websocket_port(free_port()?),
		session_opts,
	)
	.await?;
	let console = browser.console().await?;
	browser.navigate(&url).await?;

	let start = Instant::now();
	let code = loop {
		for entry in console.drain() {
			cross_log!("{}", entry.text);
		}
		let exit = browser.evaluate_value("globalThis.__beet_exit").await?;
		if let Some(code) = exit.as_i64() {
			break code;
		}
		if start.elapsed() > SUITE_DEADLINE {
			bevybail!(
				"wasm browser module gave no verdict within {SUITE_DEADLINE:?}"
			);
		}
		time_ext::sleep(Duration::from_millis(100)).await;
	};
	// the verdict can race the last console frames over the socket
	time_ext::sleep(Duration::from_millis(200)).await;
	for entry in console.drain() {
		cross_log!("{}", entry.text);
	}
	browser.kill().await?;

	// stop the server: removing the parked `Running` fires its teardown
	cx.world()
		.with(move |world: &mut World| {
			world
				.entity_mut(root)
				.remove_recursive::<Children, Running<Response>>();
		})
		.await;

	if code != 0 {
		bevybail!("wasm browser module exited with {code}");
	}
	Ok(())
}

/// The runner-provided env the host page serves back to `env_var` probes.
fn env_json() -> String {
	format!(
		"{{\"WORKSPACE_ROOT\": \"{}\"}}",
		fs_ext::workspace_root()
			.to_string_lossy()
			.replace('\\', "\\\\")
			.replace('"', "\\\"")
	)
}

/// Runner args to url query params: `--flag[=value]` becomes `flag[=value]`,
/// and a positional name filter becomes `include=<filter>`, since a path
/// segment would itself parse as a positional and filter everything out.
fn args_to_query(args: &[String]) -> String {
	let pairs = args
		.iter()
		.map(|arg| match arg.strip_prefix("--") {
			Some(flag) => flag.to_string(),
			None => format!("include={arg}"),
		})
		.collect::<Vec<_>>();
	match pairs.is_empty() {
		true => String::new(),
		false => format!("?{}", pairs.join("&")),
	}
}

/// An OS-assigned free port for the driver process (bind, read, drop). Racy
/// in principle, fine for a runner that owns the machine's test run. Shared
/// with the `wasm_render_check` smoketest's driver.
pub(crate) fn free_port() -> Result<u16> {
	TcpListener::bind("127.0.0.1:0")?.local_addr()?.port().xok()
}
