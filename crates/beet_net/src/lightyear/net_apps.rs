use crate::prelude::*;
use lightyear_common::apps::default_settings;
use lightyear_common::apps::Apps;

pub fn net_apps() -> Apps {
	let cli = lightyear_common::apps::cli();
	// build the bevy app (this adds common plugin such as the DefaultPlugins)
	// and returns the `ClientConfig` and `ServerConfig` so that we can modify them if needed
	let mut apps = Apps::from_cli(default_settings(), cli);
	// add our plugins
	apps.add_plugins(BaseClientPlugin, BaseServerPlugin, BaseSharedPlugin);
	// run the app
	apps
	// app.run();
}
