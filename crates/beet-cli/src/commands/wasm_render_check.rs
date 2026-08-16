//! The committed page-driving check for the browser render boot, the durable
//! form of the throwaway script that verified the wasm render work: serves the
//! built `beet-render.wasm` at a page generated through the real `<Wasm>` /
//! `<MainBsx>` templates, boots it in headless chromium through the in-house
//! webdriver, and asserts both sides of the canvas design:
//!
//! - the WebGPU boot (chrome granted an adapter via `--enable-unsafe-webgpu
//!   --use-angle=gl`, a SwiftShader device in headless CI) claims a canvas,
//!   stamps `data-raw-handle` and draws real pixels into it;
//! - the GPU-less boot (default headless chrome, no adapter) still runs the
//!   whole render stack but stays surfaceless: no canvas is claimed.
//!
//! Ignored by default since it needs artifacts and a browser on PATH:
//!
//! ```sh
//! just build-wasm-render   # the artifact under test
//! just check-wasm-render   # this check (needs chromedriver + a chromium)
//! ```

use crate::commands::run_wasm_browser::free_port;
use beet::prelude::webdriver::*;
use beet::prelude::*;

/// The program the page runs: the smallest windowed wgpu scene.
const SCENE_SRC: &str = "/examples/spatial/scene_3d.bsx";
/// The artifact under test, built by `just build-wasm-render`.
const WASM_SRC: &str = "/assets/wasm/beet-render.wasm";

/// Read the claimed canvas back: `null` when no canvas is claimed, else
/// `[distinct_colors, width, height]`, resolved fully in-page so the test
/// never decodes a png.
const PIXEL_PROBE: &str = r#"(() => {
	const canvas = document.querySelector("canvas[data-raw-handle]");
	if (!canvas) return null;
	// only `toDataURL` snapshots a WebGPU canvas's presented frame in chrome
	// (`drawImage`/`createImageBitmap` read back blank), so round-trip through
	// an <img> to count pixels; `evaluate` awaits the promise.
	return new Promise((resolve) => {
		const img = new Image();
		img.onload = () => {
			const probe = document.createElement("canvas");
			probe.width = img.width;
			probe.height = img.height;
			const cx = probe.getContext("2d");
			cx.drawImage(img, 0, 0);
			const data = cx.getImageData(0, 0, probe.width, probe.height).data;
			const colors = new Set();
			for (let i = 0; i < data.length; i += 4) {
				colors.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2]);
			}
			resolve([colors.size, canvas.width, canvas.height]);
		};
		img.src = canvas.toDataURL();
	});
})()"#;

/// The check page, rendered through the real templates so the loader contract
/// under test is the one the served `/render` page uses. No canvas is supplied,
/// exercising the created-and-appended path (`/render` covers page-supplied).
fn render_page() -> Result<String> {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	let root = world
		.spawn(children![
			MainBsx {
				src: SCENE_SRC.into(),
			}
			.into_snippet_bundle(),
			Wasm {
				src: WASM_SRC.into(),
				js: default(),
			}
			.into_snippet_bundle(),
		])
		.flush();
	let body = HtmlRenderer::new()
		.render(&mut RenderContext::new(root, &mut world))?
		.to_string();
	format!(
		"<!doctype html>\n<html><head><meta charset=\"utf-8\"/></head><body>{body}</body></html>"
	)
	.xok()
}

/// Serve the check page at `/` plus the workspace's `assets/` and `examples/`
/// on an ephemeral in-process `HttpServer` (its own app thread, mirroring
/// `run_wasm_browser`'s server shape), returning the bound port.
async fn serve(page: String) -> Result<u16> {
	let check_dir = AbsPathBuf::new_workspace_rel("target/wasm-render-check")?;
	fs_ext::write(check_dir.join("index.html"), &page)?;
	let workspace = FsStore::new(AbsPathBuf::new_workspace_rel("")?);
	let page_store = FsStore::new(check_dir);
	std::thread::spawn(move || {
		let mut app = App::new();
		// RouterPlugin pulls ServerPlugin itself
		app.add_plugins((MinimalPlugins, AsyncPlugin, RouterPlugin));
		let root = app
			.world_mut()
			.spawn((
				HttpServer {
					port: Some(0),
					..default()
				},
				workspace,
				children![(
					Router::default(),
					children![
						// the page rides its own static mount (a root greedy mount
						// cannot mix with the static `assets`/`examples` prefixes);
						// the extensionless rule resolves `/page` to `index.html`
						(
							ServeBlobs {
								prefix: "page".into(),
								cache: default(),
							}
							.into_snippet_bundle(),
							page_store
						),
						// the artifact and the scene program, from the ancestor
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
				)],
			))
			.id();
		app.world_mut()
			.entity_mut(root)
			.run_async_local(move |server| async move {
				server
					.call::<Request, Response>(Request::from_cli_str(
						"--server=http",
					))
					.await?;
				Ok(())
			});
		app.run();
	});
	super::wait_for_port().await
}

/// A uniquely-ported chromium driver, so the check never fights another suite's.
fn driver() -> Result<Client> {
	Client::default()
		.with_driver_port(free_port()?)
		.with_websocket_port(free_port()?)
		.xok()
}

/// Drain the console into `log`, streaming each entry for the person watching.
fn drain(console: &Collector<ConsoleEntry>, log: &mut String) {
	for entry in console.drain() {
		cross_log!("{}", entry.text);
		log.push_str(&entry.text);
		log.push('\n');
	}
}

#[beet::test(timeout_ms = 300_000)]
#[ignore = "smoketest: needs `just build-wasm-render` + chromedriver"]
async fn browser_render_boot() {
	if !AbsPathBuf::new_workspace_rel("assets/wasm/beet-render.wasm")
		.map(|path| path.exists())
		.unwrap_or_default()
	{
		panic!("missing artifact, run `just build-wasm-render`");
	}

	let port = serve(render_page().unwrap()).await.unwrap();
	let url = format!("http://127.0.0.1:{port}/page");

	// -- the WebGPU boot claims a canvas and draws --
	let mut browser = Browser::new_with_opts(
		driver().unwrap(),
		NewSessionOptions::default()
			.with_disable_gpu(false)
			// `--use-angle=gl` is load-bearing: chrome's headless Vulkan path
			// (`--enable-features=Vulkan`, the item-35 flag set) instantly
			// destroys every created device on this chromium, plain JS pages
			// included; the gl-backed SwiftShader path renders fine
			.with_extra_args(vec![
				"--enable-unsafe-webgpu".into(),
				"--use-angle=gl".into(),
			]),
	)
	.await
	.unwrap();
	let console = browser.console().await.unwrap();
	browser.navigate(&url).await.unwrap();

	// the module fetch, adapter probe, app boot and first presented frame
	let mut log = String::new();
	let deadline = Instant::now() + Duration::from_secs(180);
	let (colors, width, height) = loop {
		drain(&console, &mut log);
		let probe = browser.evaluate_value(PIXEL_PROBE).await.unwrap();
		if let Some(result) = probe.as_array() {
			// the canvas is claimed; break once pixels are nonuniform (the
			// first frames may still be clear-color only)
			let colors = result[0].as_i64().unwrap_or_default();
			if colors > 1 {
				break (
					colors,
					result[1].as_i64().unwrap_or_default(),
					result[2].as_i64().unwrap_or_default(),
				);
			}
		}
		if Instant::now() > deadline {
			panic!("no drawn canvas within deadline. console:\n{log}");
		}
		time_ext::sleep(Duration::from_millis(500)).await;
	};
	drain(&console, &mut log);
	log.xnot().xpect_contains("panicked");
	// the 3d scene draws many shades (sky, lit ground, shadow), so a low bar
	// still separates a real render from a cleared or garbage surface
	if colors <= 8 {
		panic!("only {colors} distinct colors in a {width}x{height} canvas");
	}
	(width > 0 && height > 0).xpect_true();
	browser.kill().await.unwrap();

	// -- the GPU-less boot runs but stays surfaceless --
	let mut browser = Browser::new_with_opts(driver().unwrap(), default())
		.await
		.unwrap();
	let console = browser.console().await.unwrap();
	browser.navigate(&url).await.unwrap();
	// boot evidence: the render stack logs on the way up (GPU-less warnings at
	// minimum), so wait for the console to speak, settle, then assert
	let mut log = String::new();
	let deadline = Instant::now() + Duration::from_secs(60);
	while log.is_empty() {
		drain(&console, &mut log);
		if Instant::now() > deadline {
			panic!("gpu-less boot produced no console output");
		}
		time_ext::sleep(Duration::from_millis(500)).await;
	}
	time_ext::sleep(Duration::from_secs(5)).await;
	drain(&console, &mut log);
	log.xnot().xpect_contains("panicked");
	browser
		.evaluate_value(PIXEL_PROBE)
		.await
		.unwrap()
		// a claimed canvas here means the surfaceless constraint regressed
		.is_null()
		.xpect_true();
	browser.kill().await.unwrap();
}
