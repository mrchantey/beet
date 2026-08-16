//! The browser host for the wasm test runner, selected by
//! `BEET_WASM_HOST=browser`: where the deno host gives a wasm suite a js
//! runtime with fs access but no DOM, this serves the `wasm-bindgen` output
//! over loopback http (ES modules and wasm fetch do not load over `file://`),
//! boots a headless chromium through the in-house webdriver, streams the
//! page's console to stdout, and exits with the suite's verdict.
//!
//! The page needs almost nothing from us: runner args arrive as url query
//! params (the wasm arg fallback already parses `location`), so the generated
//! `index.html` installs only the required globals before `init()` runs the
//! suite: the `catch_no_abort_inner` passthrough (without which the first
//! panicking test traps the module), `exit` writing the verdict to
//! `globalThis.__beet_exit` for this driver to poll, and a `WORKSPACE_ROOT`
//! env probe. No fs shims are installed: snapshot and store tests stay deno
//! and native territory, browser-marked tests are DOM tests.
//!
//! Unlike the deno host, no [`BootstrapConfig`] is delivered: a browser suite
//! runs only `#[beet_core::test(browser)]` DOM tests, which read no stage or
//! store config.

use beet::net::prelude::webdriver::*;
use beet::prelude::*;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

/// How long the whole suite may run before the driver gives up. Generous: the
/// per-test timeouts inside the runner are the real watchdog, this only
/// catches a suite that never reports at all.
const SUITE_DEADLINE: Duration = Duration::from_secs(600);

/// Serve `runner_dir`, drive a headless browser at it, stream the console and
/// propagate the suite's exit code.
pub(crate) async fn run(runner_dir: &Path, args: Vec<String>) -> Result {
	let index = index_html(&fs_ext::workspace_root().to_string_lossy());
	fs_ext::write_async(runner_dir.join("index.html"), index).await?;
	let server_port = serve_dir(runner_dir.to_path_buf())?;
	let url =
		format!("http://127.0.0.1:{server_port}/{}", args_to_query(&args));

	let driver_port = free_port()?;
	let process = ClientProcess::new_with_opts(
		Client::default()
			.with_driver_port(driver_port)
			.with_websocket_port(free_port()?),
	)?;
	let session = process.client().new_session().await?;
	let mut page = Page::from_session(session).await?;
	// console attaches before navigation so the suite's first line is caught
	let console = page.console().await?;
	page.navigate(&url).await?;

	let start = Instant::now();
	let code = loop {
		for entry in console.drain() {
			cross_log!("{}", entry.text);
		}
		let exit = page.evaluate_value("globalThis.__beet_exit").await?;
		if let Some(code) = exit.as_i64() {
			break code;
		}
		if start.elapsed() > SUITE_DEADLINE {
			bevybail!(
				"wasm browser suite gave no verdict within {SUITE_DEADLINE:?}"
			);
		}
		time_ext::sleep(Duration::from_millis(100)).await;
	};
	// the verdict can race the last console frames over the socket
	time_ext::sleep(Duration::from_millis(200)).await;
	for entry in console.drain() {
		cross_log!("{}", entry.text);
	}

	page.kill().await?;
	process.kill()?;
	if code != 0 {
		bevybail!("wasm browser suite exited with {code}");
	}
	Ok(())
}

/// The host page: installs the runner globals, then `init()` runs the suite.
fn index_html(workspace_root: &str) -> String {
	// a js string literal of the path (quote/backslash escape suffices)
	let root_json = format!(
		"\"{}\"",
		workspace_root.replace('\\', "\\\\").replace('"', "\\\"")
	);
	format!(
		r#"<!doctype html>
<html>
<head><meta charset="utf-8"/><title>beet wasm test host</title></head>
<body>
<script type="module">
// runner globals, installed before init() runs the suite. Each also under the
// `test_` alias: a `--lib` test build links beet_core twice, so the bindgen
// glue imports both names.
const exit = (code) => {{ globalThis.__beet_exit = code; }};
const passthrough = (func) => func();
const env = {{ WORKSPACE_ROOT: {root_json} }};
const env_var = (key) => env[key] ?? null;
Object.assign(globalThis, {{
	exit, test_exit: exit,
	catch_no_abort_inner: passthrough, test_catch_no_abort_inner: passthrough,
	env_var, test_env_var: env_var,
}});
import init from "./bindgen.js";
try {{
	// runs the whole suite; the verdict arrives via the exit global
	await init();
}} catch (err) {{
	console.error("wasm init failed:", err);
	exit(101);
}}
</script>
</body>
</html>
"#
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

/// An OS-assigned free port (bind, read, drop). Racy in principle, fine for
/// a runner that owns the machine's test run.
fn free_port() -> Result<u16> {
	TcpListener::bind("127.0.0.1:0")?
		.local_addr()?
		.port()
		.xok()
}

/// Serve `dir` on an ephemeral loopback port: a std accept loop on a detached
/// thread, one thread per connection, plenty for one test page's requests.
/// Static file serving only lives for the runner's lifetime, so no shutdown
/// plumbing: the process exit reaps it.
fn serve_dir(dir: PathBuf) -> Result<u16> {
	let listener = TcpListener::bind("127.0.0.1:0")?;
	let port = listener.local_addr()?.port();
	std::thread::spawn(move || {
		for stream in listener.incoming() {
			let Ok(stream) = stream else { continue };
			let dir = dir.clone();
			std::thread::spawn(move || {
				let _ = handle_conn(stream, &dir);
			});
		}
	});
	Ok(port)
}

/// Answer one `GET`: resolve the path under `dir` (`/` is `index.html`),
/// refuse traversal, serve with the module-correct mime.
fn handle_conn(stream: TcpStream, dir: &Path) -> Result {
	let mut reader = BufReader::new(stream.try_clone()?);
	let mut request_line = String::new();
	reader.read_line(&mut request_line)?;
	// drain the headers so the browser sees a well-behaved close
	for line in reader.by_ref().lines() {
		if line.map(|line| line.trim().is_empty()).unwrap_or(true) {
			break;
		}
	}
	let path = request_line
		.split_whitespace()
		.nth(1)
		.and_then(|target| target.split('?').next())
		.unwrap_or("/")
		.trim_start_matches('/');
	let rel = match path.is_empty() {
		true => "index.html",
		false => path,
	};
	let mut stream = stream;
	if rel.contains("..") {
		return respond(&mut stream, 404, "text/plain", b"not found");
	}
	match std::fs::read(dir.join(rel)) {
		Ok(body) => respond(&mut stream, 200, content_type(rel), &body),
		Err(_) => respond(&mut stream, 404, "text/plain", b"not found"),
	}
}

fn respond(
	stream: &mut TcpStream,
	status: u16,
	content_type: &str,
	body: &[u8],
) -> Result {
	let reason = match status {
		200 => "OK",
		_ => "Not Found",
	};
	stream.write_all(
		format!(
			"HTTP/1.1 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
			body.len()
		)
		.as_bytes(),
	)?;
	stream.write_all(body)?;
	Ok(())
}

/// Module scripts require a correct js mime; the rest is best-effort.
fn content_type(path: &str) -> &'static str {
	match path.rsplit('.').next() {
		Some("html") => "text/html",
		Some("js") | Some("mjs") => "text/javascript",
		Some("wasm") => "application/wasm",
		_ => "application/octet-stream",
	}
}
