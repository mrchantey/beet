use crate::beet::prelude::*;
use crate::prelude::*;
use beetmash::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;


pub fn spawn_ik_camera(mut commands: Commands) {
	commands.spawn((
		Name::new("Camera"),
		BundlePlaceholder::Camera3d,
		Transform::from_xyz(0., 7., 7.).looking_at(Vec3::ZERO, Vec3::Y),
	));
}

pub fn spawn_arm_with_keyboard_target(mut commands: Commands) {
	let target = spawn_keyboard_target(&mut commands);
	commands.spawn((
		Name::new("scene"),
		BundlePlaceholder::Gltf("robot-arm/robot-arm.glb".into()),
		Transform::from_scale(Vec3::splat(10.)),
		TargetAgent(target),
		IkSpawner::default(),
	));
}

fn spawn_keyboard_target(commands: &mut Commands) -> Entity {
	commands
		.spawn((
			Name::new("Target"),
			KeyboardController::default(),
			// FollowCursor3d::ORIGIN_Z,
			Transform::from_xyz(0., 1.5, 2.5).looking_to(-Vec3::Z, Vec3::Y),
			BundlePlaceholder::Pbr {
				mesh: Sphere::new(0.2).into(),
				material: MaterialPlaceholder::unlit(tailwind::BLUE_500),
			},
		))
		.id()
}


pub fn spawn_test_arm(mut commands: Commands) {
	let target = spawn_keyboard_target(&mut commands);

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
			.spawn((Transform::default(), BundlePlaceholder::Pbr {
				mesh: Cylinder::new(0.2, 0.1).into(),
				material: MaterialPlaceholder::unlit(tailwind::AMBER_100),
			}))
			.id();
	});

	commands.entity(entity_base).with_children(|parent| {
		entity_segment1 = ik_segment(
			&mut parent.spawn_empty(),
			&ik_solver.segment1,
			Transform::default(),
			arm_width,
			tailwind::AMBER_300,
		);
	});

	commands.entity(entity_segment1).with_children(|parent| {
		entity_segment2 = ik_segment(
			&mut parent.spawn_empty(),
			&ik_solver.segment2,
			Transform::from_xyz(0., 0., -ik_solver.segment1.len),
			arm_width,
			tailwind::AMBER_500,
		);
	});

	commands.entity(entity_segment2).with_children(|parent| {
		entity_segment3 = ik_segment(
			&mut parent.spawn_empty(),
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


pub fn ik_segment(
	commands: &mut EntityCommands,
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
				BundlePlaceholder::Pbr {
					mesh: Cuboid::new(arm_width, arm_width, seg.len).into(),
					material: MaterialPlaceholder::unlit(color),
				},
			));
		})
		.id()
}
