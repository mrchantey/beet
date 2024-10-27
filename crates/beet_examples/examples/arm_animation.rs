use beet_examples::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use beetmash::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use forky::prelude::TransformExt;
use scenes::spawn_arm;
use std::time::Duration;

pub fn main() {
	App::new()
		.add_plugins(crate_test_beet_example_plugin)
		.add_systems(
			Startup,
			(
				beetmash::core::scenes::lighting_3d,
				beetmash::core::scenes::ground_3d,
				beet_examples::scenes::flow::beet_debug_start_and_stop,
				beet_examples::emote_agent::scenes::spawn_ik_camera,
				setup,
			),
		)
		.run();
}


fn setup(mut commands: Commands) {
	let mut target = Entity::PLACEHOLDER;
	commands
		// target parent is used to define offset transform
		.spawn((
			Name::new("Target Parent"),
			Transform::from_xyz(0., 1.5, 0.)
				.with_scale_value(2.)
				.looking_to(Dir3::NEG_Y, Dir3::NEG_Z),
			// .with_rotation_x(PI),
		))
		.with_children(|parent| {
			target = parent
				.spawn((Name::new("Target"), BundlePlaceholder::Pbr {
					mesh: Sphere::new(0.2).into(),
					material: MaterialPlaceholder::unlit(tailwind::BLUE_500),
				}))
				.id();
		});
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
				TargetAgent(target),
				InsertProceduralAnimation::default(),
				PlayProceduralAnimation::default()
					.with_duration(Duration::from_secs_f32(3.)),
			));
			parent.spawn((
				Name::new("Pause"),
				TriggerInDuration::with_range(
					OnRunResult::success(),
					Duration::from_secs(1)..Duration::from_secs(4),
				),
			));
		});

	spawn_arm(&mut commands, target);
}
