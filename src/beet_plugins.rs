use crate::prelude::*;

/// This plugin will add all the default plugins
/// for a *Beet* application, including the appropriate
/// runner depending on feature flags.
/// ## Note
/// This plugin must be added *after* MinimalPlugins etc to
/// ensure it has last say in setting the runner.
pub struct BeetPlugins;

impl Plugin for BeetPlugins {
	fn build(&self, app: &mut App) {
		#[cfg(feature = "rsx")]
		app.init_plugin::<ApplyDirectivesPlugin>();
		#[cfg(feature = "build")]
		app.init_plugin::<BuildPlugin>();
		#[cfg(feature = "server")]
		app.init_plugin::<LoadSnippetsPlugin>();
		app.init_plugin::<BeetRunner>();
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
		app.set_runner(LaunchConfig::runner);

		#[cfg(feature = "server")]
		app.init_plugin_with(RouterRunner::parse());

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
	info!(
		"\n🌱 Running Beet\nbinary: {binary}\n{}build: {}",
		*pkg_config,
		if cfg!(debug_assertions) {
			"debug"
		} else {
			"release"
		},
	);
}
