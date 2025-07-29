use crate::prelude::*;
use bevy::app::plugin_group;

plugin_group! {
/// This plugin group will add all the default plugins for a *Beet* application:
pub struct BeetPlugins {
	#[cfg(feature = "rsx")]
	:TemplatePlugin,
	#[cfg(feature = "server")]
	:RouterPlugin,
	#[cfg(feature = "client")]
	:ClientPlugin,
	#[cfg(feature = "launch")]
	:LaunchPlugin,
}
}
#[allow(unused)]
#[cfg(feature = "client")]
#[derive(Default)]
struct ClientPlugin;

#[cfg(feature = "client")]
impl Plugin for ClientPlugin {
	fn build(&self, app: &mut App) { app.set_runner(ReactiveApp::runner); }
}
#[cfg(feature = "launch")]
#[derive(Default)]
struct LaunchPlugin;

#[cfg(feature = "launch")]
impl Plugin for LaunchPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(CargoManifest::load().unwrap())
			.set_runner(|mut app| {
				app.world_mut()
					.run_sequence_once(import_route_file_collection)
					.unwrap()
					.run_sequence_once(ParseFileSnippets)
					.unwrap()
					.run_sequence_once(RouteCodegenSequence)
					.unwrap()
					.run_sequence_once(export_route_codegen)
					.unwrap();
				AppExit::Success
			});
	}
}
