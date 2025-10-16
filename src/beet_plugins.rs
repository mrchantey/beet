use crate::prelude::*;
use bevy::app::plugin_group;

plugin_group! {
/// This plugin group will add all the default plugins for a *Beet* application:
pub struct BeetPlugins {
	#[cfg(feature = "rsx")]
	:ApplyDirectivesPlugin,
	#[cfg(feature = "build")]
	:BuildPlugin,
	:BeetRunner,
}
}


/// Will set the [`App::runner`] based on the features enabled.
#[derive(Default)]
pub struct BeetRunner;

impl Plugin for BeetRunner {
	#[allow(unused, unreachable_code)]
	fn build(&self, app: &mut App) {
		// order matters, last flag wins
		#[cfg(feature = "launch")]
		app.set_runner(LaunchRunner::runner);

		#[cfg(feature = "server")]
		app.set_runner(ServerRunner::runner);

		#[cfg(feature = "client")]
		app.set_runner(ReactiveApp::runner);

		app.add_systems(Startup, print_config);
		#[cfg(not(any(
			feature = "launch",
			feature = "server",
			feature = "client"
		)))]
		panic!(
			"No runner feature enabled. Please enable one of: launch, server, client."
		);
	}
}

#[allow(unused)]
fn print_config(pkg_config: Res<PackageConfig>) {
	#[cfg(feature = "launch")]
	let binary = "Launch";
	#[cfg(feature = "server")]
	let binary = "Server";
	#[cfg(feature = "client")]
	let binary = "Client";

	#[cfg(any(feature = "launch", feature = "server", feature = "client"))]
	info!("\n🌱 Running Beet\nbinary: {binary}\n{}", *pkg_config);
}
