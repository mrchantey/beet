use beet::examples::scenes;
use beet::prelude::*;
use bevy::color::palettes::tailwind;

fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				scenes::lighting_3d,
				scenes::ground_3d,
				spawn_ik_camera,
				spawn_arm_with_keyboard_target,
			),
		)
		.run();
}




fn spawn_ik_camera(mut commands: Commands) {
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		Transform::from_xyz(0., 7., 7.).looking_at(Vec3::ZERO, Vec3::Y),
	));
}

fn spawn_arm_with_keyboard_target(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let target =
		spawn_keyboard_target(&mut commands, &mut meshes, &mut materials);
	commands.spawn((
		Name::new("scene"),
		SceneRoot(asset_server.load(
			GltfAssetLabel::Scene(0).from_asset("robot-arm/robot-arm.glb"),
		)),
		Transform::from_scale(Vec3::splat(10.)),
		TargetEntity::Other(target),
		IkSpawner::default(),
	));
}

fn spawn_keyboard_target(
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
	commands
		.spawn((
			Name::new("Target"),
			KeyboardController::default(),
			// FollowCursor3d::ORIGIN_Z,
			Transform::from_xyz(0., 1.5, 2.5).looking_to(-Vec3::Z, Vec3::Y),
			Mesh3d(meshes.add(Sphere::new(0.2))),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: tailwind::BLUE_500.into(),
				unlit: true,
				..Default::default()
			})),
		))
		.id()
}

#[allow(unused)]
fn spawn_test_arm(
	mut commands: Commands,
	// asset_server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let target =
		spawn_keyboard_target(&mut commands, &mut meshes, &mut materials);

	let ik_solver = IkArm4Dof::new(
		0.,
		IkSegment::DEG_360,
		IkSegment::DEG_360,
		IkSegment::DEG_360.with_len(0.2),
	);
	let arm_width = 0.1;

	let root = commands
		.spawn((
			Name::new("IK Root"),
			Transform::default().looking_to(Vec3::X, Vec3::Y),
		))
		.id();
	let mut entity_base = Entity::PLACEHOLDER;
	let mut entity_segment1 = Entity::PLACEHOLDER;
	let mut entity_segment2 = Entity::PLACEHOLDER;
	let mut entity_segment3 = Entity::PLACEHOLDER;

	commands.entity(root).with_children(|parent| {
		entity_base = parent
			.spawn((
				Transform::default(),
				Mesh3d(meshes.add(Cylinder::new(0.2, 0.1))),
				MeshMaterial3d(materials.add(StandardMaterial {
					base_color: tailwind::AMBER_100.into(),
					unlit: true,
					..Default::default()
				})),
			))
			.id();
	});

	commands.entity(entity_base).with_children(|parent| {
		entity_segment1 = ik_segment(
			&mut parent.spawn_empty(),
			&mut meshes,
			&mut materials,
			&ik_solver.segment1,
			Transform::default(),
			arm_width,
			tailwind::AMBER_300,
		);
	});

	commands.entity(entity_segment1).with_children(|parent| {
		entity_segment2 = ik_segment(
			&mut parent.spawn_empty(),
			&mut meshes,
			&mut materials,
			&ik_solver.segment2,
			Transform::from_xyz(0., 0., -ik_solver.segment1.len),
			arm_width,
			tailwind::AMBER_500,
		);
	});

	commands.entity(entity_segment2).with_children(|parent| {
		entity_segment3 = ik_segment(
			&mut parent.spawn_empty(),
			&mut meshes,
			&mut materials,
			&ik_solver.segment2,
			Transform::from_xyz(0., 0., -ik_solver.segment1.len),
			arm_width,
			tailwind::AMBER_700,
		);
	});

	commands.spawn((
		Name::new("IK Solver"),
		IkArm4DofTransforms::new(
			ik_solver,
			target,
			entity_base,
			entity_segment1,
			entity_segment2,
			entity_segment3,
		),
	));
}


fn ik_segment(
	commands: &mut EntityCommands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<StandardMaterial>>,
	seg: &IkSegment,
	transform: Transform,
	arm_width: f32,
	color: Srgba,
) -> Entity {
	commands
		.insert((Name::new("Segment"), transform, Visibility::Visible))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Mesh"),
				Transform::from_xyz(0., 0., -seg.len * 0.5),
				Mesh3d(meshes.add(Cuboid::new(arm_width, arm_width, seg.len))),
				MeshMaterial3d(materials.add(StandardMaterial {
					base_color: color.into(),
					unlit: true,
					..Default::default()
				})),
			));
		})
		.id()
}
