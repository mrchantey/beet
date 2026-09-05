//! The `beet` binary: discover an entry, load it as a live build, let the loaded
//! tree run itself, and exit unless something kept it alive.
//!
//! All of that is plugin composition shared with every other beet binary.
//! What is left here is what only this binary can own: the two target
//! entrypoints (native `main`, the wasm exported [`start`]), its own compiled
//! feature surface, the dev-command capabilities, and the windowed render
//! path's window lifecycle.
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

/// The one app body every target runs: the trusted defaults and entry loader,
/// plus what only this binary supplies: the dev-command capabilities, its own
/// compiled feature surface, and the windowed render path's window lifecycle.
fn build_app() -> App {
	let mut app = App::new();
	app.add_plugins((BeetPlugins, cli_plugins, LaunchPlugin));
	// this binary's cargo features, spawned before the entry loads so its
	// `<CrateCheck/>` and any `--features` verify against them. The primary
	// registration, ie the one an unprefixed requirement resolves to.
	app.world_mut().spawn(registration::cli());
	// the windowed render path's window lifecycle + screenshot harness. The
	// facade's `BeetPlugins` links winit windowless (a capability, not a window);
	// the binary owns the lifecycle (continuous updates, escape/close-to-exit,
	// `BEET_SCREENSHOT` capture), so a data-spawned `<Window/>` appears and a
	// headless `.bsx` keeps running under the render binary.
	#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
	app.add_plugins(render_window_plugin);
	app
}

/// The capabilities this binary links on top of the trusted defaults: the
/// native-only dev commands, inert until a `main.bsx` names them, and the one
/// command among them that runs ANOTHER program, whose `--main`/`--repo` the
/// loader must therefore leave alone.
fn cli_plugins(app: &mut App) {
	#[cfg(not(target_arch = "wasm32"))]
	app.add_plugins(CliCommandsPlugin)
		.insert_resource(ArgvPassthrough::new([RunWasm::COMMAND]));
	// the wasm binary is a module a runner HOSTS, never the runner itself
	#[cfg(target_arch = "wasm32")]
	let _ = app;
}
