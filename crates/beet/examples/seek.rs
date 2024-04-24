use beet::prelude::*;
use bevy::prelude::*;


fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(DefaultPlugins)
		.add_plugins(DefaultBeetPlugins::<CoreModule>::default())
		.add_systems(Startup,setup)
		.run()
	/*-*/;
}


fn setup(mut commands: Commands) {
	commands.spawn(Camera3dBundle {
		transform: Transform::from_translation(Vec3::new(0., 1., 0.))
			.looking_at(Vec3::ZERO, Vec3::Y),
		..default()
	});
}
