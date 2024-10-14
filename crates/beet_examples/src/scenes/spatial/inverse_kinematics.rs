use crate::prelude::FollowCursor3d;
use beet_spatial::prelude::Ik2Dof;
use beet_spatial::prelude::Ik2DofTransforms;
use beet_spatial::prelude::IkSegment;
// use crate::beet::prelude::*;
// use beetmash::core::scenes::Foxie;
use beetmash::prelude::*;
// use bevy::animation::RepeatAnimation;
use bevy::{
	color::palettes::tailwind,
	prelude::*,
};
// use std::time::Duration;


pub fn inverse_kinematics(mut commands: Commands) {
	let ik_solver = Ik2Dof::new(IkSegment::DEG_360, IkSegment::DEG_360);
	let arm_width = 0.1;

	commands.spawn((
		Name::new("Camera"),
		BundlePlaceholder::Camera3d,
		Transform::from_xyz(0., 0., 5.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	let target = commands
		.spawn((
			Name::new("Mouse"),
			FollowCursor3d::ORIGIN_Z,
			Transform::default().looking_to(-Vec3::Z, Vec3::Y),
			BundlePlaceholder::Pbr {
				mesh: Circle::new(0.2).into(),
				material: MaterialPlaceholder::unlit(tailwind::BLUE_500),
			},
		))
		.id();


	let root = commands
		.spawn((
			Name::new("IK Root"),
			Transform::default().looking_to(Vec3::X, Vec3::Y),
		))
		.id();
	let mut entity1 = Entity::PLACEHOLDER;
	let mut entity2 = Entity::PLACEHOLDER;

	commands.entity(root).with_children(|parent| {
		entity1 = ik_segment(
			&mut parent.spawn_empty(),
			&ik_solver.segment1,
			Transform::default(),
			arm_width,
		);
	});

	commands.entity(entity1).with_children(|parent| {
		entity2 = ik_segment(
			&mut parent.spawn_empty(),
			&ik_solver.segment2,
			Transform::from_xyz(0., 0., -ik_solver.segment1.len),
			arm_width,
		);
	});

	commands.spawn((
		Name::new("IK Solver"),
		Ik2DofTransforms::new(ik_solver, target, entity1, entity2),
	));
}


pub fn ik_segment(
	commands: &mut EntityCommands,
	seg: &IkSegment,
	transform: Transform,
	arm_width: f32,
) -> Entity {
	commands
		.insert((Name::new("Segment"), transform, Visibility::Visible))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Mesh"),
				Transform::from_xyz(0., 0., -seg.len * 0.5),
				BundlePlaceholder::Pbr {
					mesh: Cuboid::new(arm_width, arm_width, seg.len).into(),
					material: MaterialPlaceholder::unlit(tailwind::AMBER_500),
				},
			));
		})
		.id()
}
