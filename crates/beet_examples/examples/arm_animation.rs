use beet_examples::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use beetmash::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
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
	let target = commands
		.spawn((
			Name::new("Target"),
			Transform::from_xyz(0., 1.5, 2.5).looking_to(-Vec3::Z, Vec3::Y),
			BundlePlaceholder::Pbr {
				mesh: Sphere::new(0.2).into(),
				material: MaterialPlaceholder::unlit(tailwind::BLUE_500),
			},
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Behavior"),
					RunOnSpawn,
					Repeat::default(),
					SequenceFlow,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("New Pos"),
						PlayProceduralAnimation::default()
							.with_duration(Duration::from_secs_f32(3.)),
						InsertProceduralAnimation::default(),
						TargetAgent(agent),
						Transform::from_xyz(0., 1., 2.)
							.with_scale(Vec3::splat(0.5)),
					));
					parent.spawn((
						Name::new("Pause"),
						TriggerInDuration::new(
							OnRunResult::success(),
							Duration::from_secs(3),
						),
					));
				});
		})
		.id();

	spawn_arm(&mut commands, target);
}
