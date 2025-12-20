#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			CliPlugin,
			LogPlugin::default(),
			DebugFlowPlugin::default(),
		))
		.try_set_error_handler(bevy::ecs::error::panic)
		.add_systems(Startup, cli_routes)
		.run();
}


fn cli_routes(mut commands: Commands) { commands.spawn(default_cli_router()); }
