use beet::prelude::*;
use beetmash::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use forky::prelude::TransformExt;
use std::time::Duration;


pub fn emote_arm(mut commands: Commands) {
	let mut target = Entity::PLACEHOLDER;
	let pos_happy = Vec3::new(0., 3., 0.);
	let pos_idle = Vec3::new(0., 2., 0.);

	let transform_idle = Transform::from_translation(pos_idle)
		.with_scale_value(2.)
		.looking_to(Dir3::NEG_Y, Dir3::X);



	let target_parent = commands
		// target parent is used to define offset transform
		.spawn((Name::new("Target Parent"), transform_idle))
		.with_children(|parent| {
			target = parent
				.spawn((Name::new("Target"), BundlePlaceholder::Pbr {
					mesh: Sphere::new(0.2).into(),
					material: MaterialPlaceholder::unlit(tailwind::BLUE_500),
				}))
				.id();
		})
		.id();


	commands.spawn((
		Name::new("Emote Arm"),
		BundlePlaceholder::Gltf("robot-arm/robot-arm-phone.glb".into()),
		Transform::from_scale(Vec3::splat(10.)),
		TargetAgent(target),
		IkSpawner::default(),
	));

	commands
		.spawn((
			Name::new("Behavior"),
			RunOnSpawn,
			Repeat::default(),
			SequenceFlow,
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("New Pos"),
				InsertOnRun::new(transform_idle).with_target(target_parent),
				TargetAgent(target),
				SetCurveOnRun::default(),
				PlayProceduralAnimation::default()
					// .with_meter_per_second(1.),
					.with_duration_secs(2.),
			));
			parent.spawn((
				Name::new("Idle Pause"),
				TriggerInDuration::with_range(
					OnRunResult::success(),
					Duration::from_secs(1)..Duration::from_secs(4),
				),
			));
			parent.spawn((
				Name::new("Happy"),
				TargetAgent(target_parent),
				SetCurveOnRun::PingPongPause {
					target: pos_happy,
					pause: 1.,
					func: EaseFunction::CubicInOut,
				},
				PlayProceduralAnimation::default().with_duration_secs(4.),
			));
		});
}
