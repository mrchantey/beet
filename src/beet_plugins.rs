use crate::prelude::*;
use bevy::app::plugin_group;

plugin_group! {
/// This plugin group will add all the default plugins for a *Beet* application:
pub struct BeetPlugins {
	#[cfg(feature = "rsx")]
	:TemplatePlugin,
	#[cfg(feature = "build")]
	:BuildPlugin,
	#[cfg(feature = "server")]
	:RouterPlugin,
	:BeetRunner,
}
}


/// Will set the [`App::runner`] based on the features enabled.
#[derive(Default)]
pub struct BeetRunner;

impl Plugin for BeetRunner {
	fn build(&self, app: &mut App) {
		#[cfg(feature = "launch")]
		app.set_runner(LaunchRunner::runner);

		#[cfg(feature = "server")]
		app.set_runner(AppRunner::runner);

		#[cfg(feature = "client")]
		app.set_runner(ReactiveApp::runner);
	}
}
