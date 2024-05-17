use crate::prelude::*;
use lightyear_common::apps::Apps;
use lightyear_common::settings::Settings;

pub fn net_apps() -> Apps {
	let cli = lightyear_common::apps::cli();
	let settings = lightyear_common::settings::settings::<Settings>(
		include_str!("../../assets/settings.ron"),
	);
	// build the bevy app (this adds common plugin such as the DefaultPlugins)
	// and returns the `ClientConfig` and `ServerConfig` so that we can modify them if needed
	let mut apps = Apps::from_cli(settings, cli);
	// add our plugins
	apps.add_plugins(BaseClientPlugin, BaseServerPlugin, BaseSharedPlugin);
	// run the app
	apps
	// app.run();
}
