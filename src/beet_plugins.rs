use crate::prelude::*;
use bevy::app::PluginGroupBuilder;

/// The default plugin set for a *Beet* application, the trusted way to get sensible
/// defaults. It selects the runner (a real winit window with the `winit` feature,
/// else the headless 30Hz schedule loop), installs beet's tracing [`LogPlugin`] and
/// the async/exit runtime, and links the router/scene/server capabilities a served
/// or presented site needs, each gated on the relevant feature.
///
/// It is a [`PluginGroup`], so any inner plugin can be reconfigured, eg
/// `BeetPlugins.set(LogPlugin::new(Level::TRACE))`. Pairs with
/// `BeetExtraPlugins` (from `beet_extra`) for the example scenes: that group
/// adds the example capabilities and leaves the runner to this one.
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
		cfg_if! {
			if #[cfg(feature = "winit")] {
				builder = builder.add_group(winit_default_plugins());
			} else {
				builder = builder.add_group(MinimalPlugins.set(
					ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 30.0)),
				));
			}
		}

		// beet's tracing-subscriber `LogPlugin` (drop-in for bevy's, which the winit
		// branch disables), then the error handler + async/exit runtime.
		builder = builder
			.add(LogPlugin::new(Level::DEBUG))
			.add(beet_runtime_plugin);

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
		builder
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
	use bevy::asset::AssetMetaCheck;
	use bevy::window::ExitCondition;
	use bevy::window::WindowPlugin;
	DefaultPlugins
		.set(AssetPlugin {
			meta_check: AssetMetaCheck::Never,
			// resolve the assets dir from the workspace root (the nearest ancestor
			// with a `Cargo.lock`), so a render scene loads its assets regardless of
			// the process cwd and with no `BEVY_ASSET_ROOT` set.
			file_path: workspace_root()
				.join("assets")
				.to_string_lossy()
				.into_owned(),
			..default()
		})
		.set(WindowPlugin {
			primary_window: None,
			exit_condition: ExitCondition::DontExit,
			..default()
		})
		.disable::<bevy::log::LogPlugin>()
}

/// The async command runtime, app-exit handling, and the panic error handler. Uses
/// `init_plugin` so it composes with plugins that pull these in themselves.
fn beet_runtime_plugin(app: &mut App) {
	app.init_plugin::<AsyncPlugin>()
		.init_plugin::<AppExitPlugin>()
		.try_set_error_handler(bevy::ecs::error::panic);
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
