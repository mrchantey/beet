use beet::prelude::*;


fn main() {
	App::new()
		.add_plugins(InfraPlugin::default())
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {}
