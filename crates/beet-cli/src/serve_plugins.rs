//! The shared App construction both the native `beet` binary and the wasm Worker
//! entry build on: the cross-platform serve plugins, with the native-only
//! dev-command / terminal targets feature-gated off wasm.
use beet::prelude::*;

/// Add the cross-platform plugins a served beet site needs: the paced schedule
/// runner, logging, the router/scene/card-stack capabilities, and the async
/// runtime. The native binary layers its dev-command and terminal targets on top
/// via [`add_native_serve_plugins`]; the wasm Worker calls only this.
pub fn add_serve_plugins(app: &mut App) {
	app.add_plugins((
		// pace the schedule loop instead of busy-spinning (the default
		// `ScheduleRunnerPlugin` runs with no wait, pinning a core even when a
		// served site is idle). 30Hz matches the TUI's 30fps render cap and halves
		// the per-tick schedule cost vs 60Hz, so an idle task on a fractional-vCPU
		// slice stays well under the CPU autoscaling target and can scale in.
		MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
			Duration::from_secs_f64(1.0 / 30.0),
		)),
		LogPlugin::new(Level::DEBUG),
		ClientAppPlugin,
		// the device-receive meta-routes (`<SceneServer/>`).
		SceneServerPlugin,
		// the rule set a presented/served site renders with.
		material::MaterialStylePlugin::default(),
		// the stack-of-cards machinery, dormant unless a `CardDeck` is present.
		CardStackPlugin,
	))
	.init_plugin::<AsyncPlugin>();
	// the host scene-push commands (`<SceneLoad/>`, ...) drive a remote device over
	// the std http client, so they are native-only; a served wasm Worker has no
	// device to push to.
	#[cfg(not(target_arch = "wasm32"))]
	app.add_plugins(SceneManagementPlugin);
}

/// Layer the native-only serve targets onto [`add_serve_plugins`]: the dev
/// commands and the live terminal / ssh-terminal hosts. None of these build on
/// wasm, so the Worker entry skips them.
#[cfg(not(target_arch = "wasm32"))]
pub fn add_native_serve_plugins(app: &mut App) {
	use crate::commands::CliCommandsPlugin;
	// dev-command capabilities stay linked as registered types, inert until a
	// `main.bsx` names them; the binary spawns no host, route, or command.
	app.add_plugins(CliCommandsPlugin);
	// the live terminal target the `TuiServer` boots into. `init_plugin` is
	// idempotent, so `NavigatorPlugin` (already added by `ClientAppPlugin`) is not
	// added twice.
	#[cfg(feature = "tui")]
	app.init_plugin::<CharcellTuiPlugin>()
		.init_plugin::<NavigatorPlugin>()
		.init_plugin::<LivePagePlugin>();
	// the multi-tenant SSH-TUI server's per-connection behavior, so a served site
	// declaring `<.. SshTuiServer>` serves each ssh session its own terminal.
	#[cfg(feature = "ssh")]
	app.init_plugin::<SshTuiPlugin>();
}
