use crate::prelude::*;
use bevy::app::PluginGroupBuilder;

/// The default plugin set for a *Beet* application, the trusted way to get sensible
/// defaults. It selects the runner (a real winit window with the `winit` feature,
/// the winit-less browser render runner on wasm, else the headless 30Hz schedule
/// loop), installs beet's tracing [`LogPlugin`] and the async/exit runtime, and
/// links the router/scene/server capabilities a served or presented site needs,
/// each gated on the relevant feature.
///
/// It is a [`PluginGroup`], so any inner plugin can be reconfigured, eg
/// `BeetPlugins.set(LogPlugin::new(Level::TRACE))`. With the `extra` feature it
/// also adds `BeetExtraPlugin` (from `beet_extra`) for the example capabilities,
/// each self-selected by a `beet_extra` sub-feature.
///
/// ## Window (`winit` feature)
/// The render stack links as a capability, not a window: no primary window opens
/// and the loop survives with none (`ExitCondition::DontExit`). The window is data,
/// spawned by the loaded scene (a `Window` entity, eg `<Window/>`), so one binary
/// runs a windowed scene `.bsx` and a headless server `.bsx` from the same build.
/// The window lifecycle (continuous updates, escape/close-to-exit, the screenshot
/// harness) is the binary's concern, added on top of this group (eg beet-cli's
/// `render_window_plugin` under its own `winit` feature).
pub struct BeetPlugins;

impl PluginGroup for BeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>();

		// the runner. winit owns the OS event loop + main thread; without it the
		// cooperative 30Hz loop paces headless servers/tools instead of busy-spinning.
		// winit still needs a display server to build its event loop, so on a headless
		// host (WSL, CI, bare SSH) with no display we fall back to the schedule loop
		// rather than panicking: a server/tool `.bsx` runs anywhere, target-agnostic.
		// With `winit` compiled, that fallback keeps the render capabilities linked but
		// GPU-less (see [`headless_render_runner`]) so scene systems never panic.
		// A browser tab is neither native branch (winit assumes it owns the loop, and
		// a tab always "has a display"), so wasm takes its own render runner.
		cfg_if! {
			if #[cfg(all(target_arch = "wasm32", feature = "winit"))] {
				builder = builder.add_group(wasm_render_runner());
			} else if #[cfg(feature = "winit")] {
				if env_ext::has_display() {
					builder = builder.add_group(winit_default_plugins());
				} else {
					builder = builder.add_group(headless_render_runner());
				}
			} else {
				builder = builder.add_group(headless_runner());
			}
		}

		// beet's tracing-subscriber `LogPlugin` (drop-in for bevy's, which the winit
		// branch disables), then the error handler + async/exit runtime.
		builder = builder
			.add(LogPlugin::new(Level::DEBUG))
			.add(beet_runtime_plugin)
			// the tick/entity-count diagnostics, so any entry can ask for a periodic
			// performance report with a `<PerfLog/>`. Inert without one.
			.add(PerfLogPlugin);

		// the beet_ui widget library, registered as an inert capability: it registers
		// the `<Button>`/`<Form>`/`<Header>`/`<Sidebar>`/… widget templates by name and
		// the default `bx:` event/verb vocabulary (`increment`/`decrement`/…), so a
		// markup-only site that uses live widgets or reactive verbs resolves them. Added
		// *before* the router so its inner `BsxPlugin` registers once: the router's
		// charcell stack reaches `BsxDefaultsPlugin` through `init_plugin` (idempotent),
		// which then no-ops rather than double-adding `BsxPlugin`.
		#[cfg(feature = "ui")]
		{
			builder = builder.add(BsxDefaultsPlugin);
		}

		// the route tree / document / server / navigation observers (the former
		// `ClientAppPlugin`) plus the scene-server meta-routes and card-stack host.
		cfg_if! {
			if #[cfg(any(feature = "router", feature = "router_render"))] {
				builder = builder.add(router_plugin);
			}
		}
		// the rule set a presented/served site renders with.
		cfg_if! {
			if #[cfg(feature = "style")] {
				builder = builder.add(material::MaterialStylePlugin::default());
			}
		}
		// the host scene-push commands drive a remote device over the std http
		// client (native-only) and (de)serialize scenes through world serde.
		cfg_if! {
			if #[cfg(all(
				not(target_arch = "wasm32"),
				feature = "router",
				feature = "template_serde"
			))] {
				builder = builder.add(SceneManagementPlugin);
			}
		}
		// the live terminal target the `TuiServer` boots into.
		cfg_if! {
			if #[cfg(all(not(target_arch = "wasm32"), feature = "tui_server"))] {
				builder = builder.add(tui_server_plugin);
			}
		}
		// the multi-tenant SSH-TUI server's per-connection behaviour.
		cfg_if! {
			if #[cfg(all(not(target_arch = "wasm32"), feature = "ssh_tui"))] {
				builder = builder.add(SshTuiPlugin);
			}
		}
		// the websocket socket-server backend (the tungstenite accept loop) so a
		// markup `{SocketServer}` boots, eg the perceive-act agent. Opt-in via the
		// `tungstenite` feature so a plain server build never links it.
		cfg_if! {
			if #[cfg(all(not(target_arch = "wasm32"), feature = "tungstenite"))] {
				builder = builder.add(beet_net::sockets::SocketServerPlugin::default());
			}
		}
		// the agent-thread runtime (+ its chat UI under `ui`): beet_thread's own
		// capability, linked by the `thread` feature independent of the example
		// wiring, so a `<Thread>` entry runs without `extra`.
		#[cfg(feature = "thread")]
		{
			builder = builder.add(beet_thread::prelude::ThreadPlugin::default());
		}
		#[cfg(all(feature = "thread", feature = "ui"))]
		{
			builder = builder.add(beet_thread::prelude::ThreadUiPlugin);
		}
		// the infra deploy block/action type registrations, so a deployer entry
		// resolves every compiled deploy type by tag independent of the example
		// templates (`extra`).
		#[cfg(feature = "infra")]
		{
			builder = builder.add(beet_infra::prelude::InfraPlugin);
		}
		// the example capabilities (the spatial/render scenes, the example
		// tools/templates, the cloudflare/aws deploy example templates), each
		// self-selected by a `beet_extra` sub-feature. A regular `Plugin`, so it
		// nests in this group; its inner plugins are idempotent (`init_plugin`),
		// safe under a double-add.
		#[cfg(feature = "extra")]
		{
			builder = builder.add(beet_extra::prelude::BeetExtraPlugin);
		}
		builder
	}
}

/// The headless runner: the cooperative 30Hz schedule loop that paces servers and
/// tools without an OS event loop. Used when the `winit` feature is off, so the
/// render/asset stack is never linked and bare `MinimalPlugins` suffice.
#[cfg(not(feature = "winit"))]
fn headless_runner() -> PluginGroupBuilder {
	MinimalPlugins
		.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
			1.0 / 30.0,
		)))
		.build()
}

/// The winit-compiled fallback when no display server is reachable (headless WSL,
/// CI, bare SSH). Keeps the whole render/asset `DefaultPlugins` stack that
/// `--all-features` links — so every render resource, event and asset type still
/// exists and no scene system panics on a missing `WindowResized`/camera/asset — but:
/// - swaps winit's OS event loop (which needs a display and panics building it) for
///   the headless schedule loop, and
/// - runs the render stack GPU-less (`WgpuSettings::backends = None`), so wgpu is
///   never initialized on the displayless host (adapter creation would error).
///
/// So a CLI/server/TUI `.bsx` runs headless anywhere; a `<Window/>`/3d scene simply
/// cannot render, rather than crashing the whole binary.
#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
fn headless_render_runner() -> PluginGroupBuilder {
	use bevy::render::RenderPlugin;
	use bevy::render::settings::RenderCreation;
	use bevy::render::settings::WgpuSettings;
	winit_default_plugins()
		.disable::<bevy::winit::WinitPlugin>()
		.add(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
			1.0 / 30.0,
		)))
		.set(RenderPlugin {
			render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
				backends: None,
				..default()
			})),
			..default()
		})
}

/// The browser runner: a tab is neither native branch of the display dichotomy.
/// winit's web event loop assumes it owns the thread (never returning to the
/// awaited wasm `start`), while the schedule loop alone would strip the render
/// stack the compiled features expect. So: the full render/asset
/// `DefaultPlugins` minus `WinitPlugin`, driven by the awaited `run_async` loop
/// like every wasm beet app.
///
/// GPU through WebGPU when the host granted an adapter — the wasm entry awaits
/// [`js_runtime::probe_webgpu`] once before building the app, and this reads
/// the cached answer. Otherwise GPU-less like [`headless_render_runner`]:
/// WebGL2 can never back this boot, since its adapter only exists on a
/// canvas-backed surface and beet boots windowless. On the WebGPU boot a
/// scene-spawned `<Window/>` presents into the page: [`WasmCanvasPlugin`]
/// (`beet_core::web_utils::wasm_canvas`) resolves or creates a `#beet-canvas`
/// and inserts the `RawHandleWrapper` the render stack surfaces from, standing
/// in for winit as the window backend. The GPU-less boot stays surfaceless, cameras drawing
/// only to texture targets. The window *lifecycle* (continuous updates,
/// close-to-exit, the screenshot harness) remains the binary's concern, as on
/// native — a tab needs none of it, since the browser owns pacing and teardown.
#[cfg(all(target_arch = "wasm32", feature = "winit"))]
fn wasm_render_runner() -> PluginGroupBuilder {
	use bevy::render::RenderPlugin;
	use bevy::render::settings::Backends;
	use bevy::render::settings::RenderCreation;
	use bevy::render::settings::WgpuSettings;
	winit_default_plugins()
		.disable::<bevy::winit::WinitPlugin>()
		.set(RenderPlugin {
			render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
				backends: js_runtime::has_webgpu()
					.then_some(Backends::BROWSER_WEBGPU),
				..default()
			})),
			..default()
		})
		.add(WasmCanvasPlugin)
}

/// The [`AssetPlugin`] shared by every render-capable runner: skips `.meta`
/// lookups (beet sites ship no sidecars) and roots the assets dir per target.
/// Native resolves from the workspace root (the nearest ancestor with a
/// `Cargo.lock`) so a scene loads its assets regardless of the process cwd; a
/// browser has no filesystem, so bevy's http reader fetches the origin-rooted
/// `/assets` url the serving site mounts (eg `<AssetsDir src="assets"
/// prefix="assets"/>`).
#[cfg(feature = "winit")]
fn asset_plugin() -> AssetPlugin {
	use bevy::asset::AssetMetaCheck;
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			let file_path = "/assets".to_string();
		} else {
			let file_path = fs_ext::workspace_root()
				.join("assets")
				.to_string_lossy()
				.into_owned();
		}
	}
	AssetPlugin {
		meta_check: AssetMetaCheck::Never,
		file_path,
		..default()
	}
}

/// The configured bevy `DefaultPlugins` for a windowed beet app: skip asset meta
/// lookups (beet sites ship no `.meta` sidecars) and disable bevy's `LogPlugin` so
/// beet's tracing one replaces it.
///
/// Critically it links the render stack as a *capability*, not a window: no primary
/// window opens (`primary_window: None`) and `ExitCondition::DontExit` keeps the
/// loop alive with no window. The window comes from the loaded scene, which spawns
/// a `Window` entity (eg `<Window/>`), so one render-built binary serves both a
/// windowed scene `.bsx` and a headless server `.bsx`. The consuming binary then
/// owns the window lifecycle (continuous updates, escape/close-to-exit).
#[cfg(feature = "winit")]
fn winit_default_plugins() -> PluginGroupBuilder {
	use bevy::window::ExitCondition;
	use bevy::window::WindowPlugin;
	DefaultPlugins
		.set(asset_plugin())
		.set(WindowPlugin {
			primary_window: None,
			exit_condition: ExitCondition::DontExit,
			..default()
		})
		.disable::<bevy::log::LogPlugin>()
}

/// The async command runtime, app-exit handling, the process config assignment,
/// the crate feature check, and the fallback error handler. Uses `init_plugin` so
/// it composes with plugins that pull these in themselves.
///
/// ## The fallback handler splits by build
///
/// Only errors raised where a `Result` cannot be returned reach it: a component
/// hook, a command, a detached async task. A route failure flows back as a
/// response and sets the exit status on that path instead.
///
/// A **debug** build panics, so a misconfiguration surfaces the instant it is
/// introduced and a dev run cannot quietly carry on wrong. A **release** build
/// logs and keeps running, because the release build is the deployed server: a
/// detached analytics write that cannot reach its table must not take the site
/// down with it. The loud-once latch on `AnalyticsStore::record` is what keeps
/// that from becoming a log per event.
fn beet_runtime_plugin(app: &mut App) {
	app.init_plugin::<AsyncPlugin>()
		.init_plugin::<AppExitPlugin>()
		.init_plugin::<BootstrapPlugin>()
		.init_plugin::<CrateCheckPlugin>();
	cfg_if! {
		if #[cfg(debug_assertions)] {
			app.try_set_error_handler(bevy::ecs::error::panic);
		} else {
			app.try_set_error_handler(bevy::ecs::error::error);
		}
	}
}

/// The route tree, document sync, server exchange and navigation observers (the
/// former `ClientAppPlugin`), plus the scene-server meta-routes and the dormant
/// card-stack machinery.
#[cfg(any(feature = "router", feature = "router_render"))]
fn router_plugin(app: &mut App) {
	app.init_plugin::<DocumentPlugin>()
		.init_plugin::<RouterPlugin>()
		.init_plugin::<ServerPlugin>()
		.init_plugin::<NavigatorPlugin>()
		.add_plugins(CardStackPlugin);
	// the scene-server meta-routes load/save scenes through world serde, so they
	// are only available (and only useful) with `template_serde`.
	#[cfg(feature = "template_serde")]
	app.add_plugins(SceneServerPlugin);
}

/// The navigable charcell host the `TuiServer` boots into, plus live-page repaint.
#[cfg(all(not(target_arch = "wasm32"), feature = "tui_server"))]
fn tui_server_plugin(app: &mut App) {
	app.init_plugin::<CharcellTuiPlugin>()
		.init_plugin::<NavigatorPlugin>()
		.init_plugin::<LivePagePlugin>();
}
