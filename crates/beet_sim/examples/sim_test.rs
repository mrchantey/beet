use beet_flow::prelude::*;
use beet_sim::prelude::*;
use beet_spatial::prelude::BeetDefaultPlugins;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;

fn main() {
	let mut stat_map = StatMap::default();
	stat_map.add_stat(StatDescriptor {
		name: "Health".to_string(),
		description: "The health of the agent".to_string(),
		emoji_hexcode: "2764".to_string(),
		global_range: StatValue::range(0.0..1.),
	});
	stat_map.add_stat(StatDescriptor {
		name: "Energy".to_string(),
		description: "The energy of the agent".to_string(),
		emoji_hexcode: "26A1".to_string(),
		global_range: StatValue::range(0.0..1.),
	});
	App::new()
		.add_plugins((
			BeetmashDefaultPlugins::with_native_asset_path("../../assets"),
			DefaultPlaceholderPlugin,
			BeetDefaultPlugins,
			BeetDebugPlugin,
			BeetSimPlugin,
		))
		.add_systems(Startup, (camera, agent, cupcake, gym))
		.insert_resource(stat_map)
		.insert_resource(BeetDebugConfig::default())
		.run();
}


fn camera(mut commands: Commands) {
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		Transform::from_xyz(0., 0., 7.),
	));
}

fn orbital_pos(index: usize, total: usize) -> Vec3 {
	let angle = TAU / total as f32 * index as f32;
	let pos = Vec3::new(f32::cos(angle), f32::sin(angle), 0.);
	pos * 0.7
}


const CHILD_SCALE: Vec3 = Vec3 {
	x: 0.5,
	y: 0.5,
	z: 0.5,
};


fn agent(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((Name::new("Agent"), Emoji::new("1F600")))
		.with_children(|parent| {
			let total_children = 4;

			let energy_stat_id = stat_map.get_by_name("Energy").unwrap();
			let health_stat_id = stat_map.get_by_name("Health").unwrap();

			let agent = parent.parent_entity();
			parent.spawn((
				Name::new("Health"),
				Transform::from_translation(orbital_pos(0, total_children))
					.with_scale(CHILD_SCALE),
				health_stat_id,
				StatValue::new(1.),
			));
			parent.spawn((
				Name::new("Energy"),
				Transform::from_translation(orbital_pos(1, total_children))
					.with_scale(CHILD_SCALE),
				energy_stat_id,
				StatValue::new(1.),
			));
			parent.spawn((
				Name::new("Walk"),
				Transform::from_translation(orbital_pos(2, total_children))
					.with_scale(CHILD_SCALE),
				TargetEntity(agent),
				Walk::default(),
			));

			parent
				.spawn((
					Name::new("Behavior"),
					Emoji::new("1F5FA"),
					Transform::from_translation(orbital_pos(3, total_children))
						.with_scale(CHILD_SCALE),
					RunOnSpawn,
					ScoreFlow::default(),
					RepeatFlow::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						TargetEntity(agent),
						ScoreProvider::NEUTRAL,
					));
					parent.spawn((
						Name::new("Desire Health"),
						TargetEntity(agent),
						StatScoreProvider::new(health_stat_id),
					));
					parent.spawn((
						Name::new("Desire Energy"),
						TargetEntity(agent),
						StatScoreProvider::new(energy_stat_id)
							.in_negative_direction(),
					));
				});
		});
}


fn cupcake(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Cupcake"),
			Emoji::new("1F9C1"),
			CollectableStat::default(),
			Transform::from_xyz(-3., -1., 0.),
		))
		.with_child((
			Name::new("Health"),
			Transform::from_translation(orbital_pos(0, 2))
				.with_scale(CHILD_SCALE),
			stat_map.get_by_name("Health").unwrap(),
			StatValue::new(-0.1),
		))
		.with_child((
			Name::new("Energy"),
			Transform::from_translation(orbital_pos(1, 2))
				.with_scale(CHILD_SCALE),
			stat_map.get_by_name("Energy").unwrap(),
			StatValue::new(0.1),
		));
}

fn gym(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Gym"),
			Emoji::new("1F3CB"),
			CollectableStat::default(),
			Transform::from_xyz(3., -1., 0.),
		))
		.with_child((
			Name::new("Health"),
			Transform::from_translation(orbital_pos(0, 2))
				.with_scale(CHILD_SCALE),
			stat_map.get_by_name("Health").unwrap(),
			StatValue::new(0.1),
		))
		.with_child((
			Name::new("Energy"),
			Transform::from_translation(orbital_pos(1, 2))
				.with_scale(CHILD_SCALE),
			stat_map.get_by_name("Energy").unwrap(),
			StatValue::new(-0.1),
		));
}
