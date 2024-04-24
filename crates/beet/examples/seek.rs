use beet::prelude::*;
use bevy::prelude::*;


#[derive(Component)]
struct Cursor;

fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(DefaultPlugins)
		.add_plugins(DefaultBeetPlugins::<CoreModule>::default())
		.add_systems(Startup, setup)
		.add_systems(Update, update_cursor)
		.add_systems(Update, forky_bevy::prelude::close_on_esc)
		.run()
	/*-*/;
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn(Camera2dBundle::default());

	// background
	commands.spawn((
		SpriteBundle {
			texture: asset_server.load("space_background/Space_Stars2.png"),
			transform: Transform::from_translation(Vec3::new(0., 0., -1.))
				.with_scale(Vec3::splat(100.)),
			..default()
		},
		ImageScaleMode::Tiled {
			tile_x: true,
			tile_y: true,
			stretch_value: 0.01,
			// ..default() // stretch_value: 0.001,
		},
	));

	// target
	let target = commands
		.spawn((
			SpriteBundle {
				texture: asset_server.load("spaceship_pack/planet_6.png"),
				..default()
			},
			Cursor,
		))
		.id();

	// agent
	commands
		.spawn((
			SpriteBundle {
				texture: asset_server.load("spaceship_pack/ship_2.png"),
				..default()
			},
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_to(500.).with_target(target),
		))
		.with_children(|parent| {
			// behavior
			parent.spawn((Seek, Running, TargetAgent(parent.parent_entity())));
		});
}

fn update_cursor(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<&mut Transform, With<Cursor>>,
	windows: Query<&Window>,
	// mut gizmos: Gizmos,
) {
	let (camera, camera_transform) = camera_query.single();

	let Some(cursor_position) = windows.single().cursor_position() else {
		return;
	};

	// Calculate a world position based on the cursor's position.
	let Some(point) =
		camera.viewport_to_world_2d(camera_transform, cursor_position)
	else {
		return;
	};

	for mut transform in cursor_query.iter_mut() {
		transform.translation = point.extend(0.);
	}

	// log::info!("cursor: {:?}, point: {:?}", cursor_position, point);

	// gizmos.circle_2d(point, 10., WHITE);
}
