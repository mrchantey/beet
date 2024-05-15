use beet::prelude::*;
use beet_examples::ExamplePlugin2d;
use bevy::prelude::*;

fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(ExamplePlugin2d)
		.add_plugins(DefaultBeetPlugins)
		.add_systems(Startup, setup)
		.add_systems(Update, spawn_on_click)
		.run()
	/*-*/;
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	for _ in 0..100 {
		spawn_agent(
			&mut commands,
			&asset_server,
			Vec3::ZERO,
			// Vec3::random_in_sphere() * 500.,
		);
	}
	commands.spawn(TextBundle {
		text: Text::from_section("Click to spawn agents", TextStyle {
			font_size: 40.0,
			..default()
		}),
		style: Style {
			padding: UiRect::all(Val::Px(16.)),
			..default()
		},
		..default()
	});
}

fn spawn_on_click(
	buttons: Res<ButtonInput<MouseButton>>,
	camera_query: Query<(&Camera, &GlobalTransform)>,
	windows: Query<&Window>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	if !buttons.pressed(MouseButton::Left) {
		return;
	}

	let (camera, camera_transform) = camera_query.single();

	let Some(cursor_position) = windows.single().cursor_position() else {
		return;
	};

	let Some(point) =
		camera.viewport_to_world_2d(camera_transform, cursor_position)
	else {
		return;
	};

	let pos = point.extend(0.);

	spawn_agent(&mut commands, &asset_server, pos);
}


fn spawn_agent(
	commands: &mut Commands,
	asset_server: &AssetServer,
	position: Vec3,
) {
	commands
		.spawn((
			SpriteBundle {
				texture: asset_server.load("spaceship_pack/ship_2.png"),
				transform: Transform::from_translation(position)
					.with_scale(Vec3::splat(0.5)),
				..default()
			},
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_to(100.),
			VelocityScalar(Vec3::new(1., 1., 0.)),
			GroupSteerAgent,
		))
		.with_children(|agent| {
			// behavior
			agent.spawn((
				Running,
				RootIsTargetAgent,
				Separate::<GroupSteerAgent>::new(1.),
				Align::<GroupSteerAgent>::new(1.),
				Cohere::<GroupSteerAgent>::new(1.),
				Wander::new(0.1),
			));
		});
}
