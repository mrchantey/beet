use beet::prelude::*;
mod petes_beets;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((http_server(8337), petes_beets::stack()));
		})
		.run()
}
