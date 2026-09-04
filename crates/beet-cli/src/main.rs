//! The `beet` binary: discover an entry, load it as a live build, let the loaded
//! tree run itself, and exit unless something kept it alive.
//!
//! All of that is [`launch::app`] in the lib, so a downstream binary linking
//! capabilities of its own runs an entry identically. What is left here is what
//! only the binary can own: the two target entrypoints (native `main`, the wasm
//! exported [`start`]) and the windowed render path's window lifecycle.
use beet::prelude::*;
use beet_cli::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> AppExit {
	// load any local `.env` (eg `BEET_REMOTE_URL`) before the app starts.
	env_ext::load_dotenv().ok();
	build_app().run()
}

// the wasm entry is the exported [`start`] below, awaited explicitly by the
// host; `main` boots nothing, so nothing runs as a side effect of module init.
// It does install the panic hook, which wasm-bindgen calls during `init()`, so a
// panic *before* the host reaches `start` still reports rather than aborting mute.
#[cfg(target_arch = "wasm32")]
fn main() { console_error_panic_hook::set_once(); }

/// The wasm start fn, exported for the host to await: the same app body as
/// native, driven by `run_async` (native `run()` would busy-wait the JS event
/// loop), resolving to the process exit code. The deno runner (`deno.ts`)
/// awaits it and `Deno.exit`s with the code; the browser's `<Wasm>` loader
/// awaits a future that simply never resolves for a long-running program.
///
/// Optional from the host's side: both loaders call it only if the module
/// exports it, so they stay general wasm runners (a module without one boots
/// from its own `main` and terminates through the `exit` global).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start() -> i32 {
	// idempotent: `main` already set it during `init()`.
	console_error_panic_hook::set_once();
	// the wasm render runner branches on the WebGPU grant, which only an awaited
	// probe can answer (`navigator.gpu` may exist yet grant no adapter), so
	// resolve it before the app and its plugins are built.
	#[cfg(feature = "winit")]
	js_runtime::probe_webgpu().await;
	build_app().run_async().await.exit_code()
}

/// The one app body every target runs, ie [`launch::app`] plus the stock
/// binary's own window lifecycle.
fn build_app() -> App {
	// only the winit branch below mutates it
	#[allow(unused_mut)]
	let mut app = launch::app(());
	// the windowed render path's window lifecycle + screenshot harness. The
	// facade's `BeetPlugins` links winit windowless (a capability, not a window);
	// the binary owns the lifecycle (continuous updates, escape/close-to-exit,
	// `BEET_SCREENSHOT` capture), so a data-spawned `<Window/>` appears and a
	// headless `.bsx` keeps running under the render binary.
	#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
	app.add_plugins(render_window_plugin);
	app
}
