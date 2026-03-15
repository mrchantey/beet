use beet::prelude::*;
mod content;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((http_server(8337), content::stack()));
		})
		.run()
}
