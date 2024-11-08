use beet_flow::prelude::*;
use beet_sim::prelude::*;
use beet_spatial::prelude::BeetDefaultPlugins;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;


const STRESS: &str = "Stress";
const SELF_CONTROL: &str = "Self-Control";

fn main() {
	let stat_map = StatMap::default()
		.with_stat(StatDescriptor {
			name: SELF_CONTROL.to_string(),
			description: "Ability to make good decisions".to_string(),
			emoji_hexcode: "1F9D8".to_string(),
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(1.),
		})
		.with_stat(StatDescriptor {
			name: STRESS.to_string(),
			description: "Current stress level".to_string(),
			emoji_hexcode: "1F92F".to_string(),
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(0.),
		});
	App::new()
		.add_plugins((
			BeetmashDefaultPlugins::with_native_asset_path("../../assets"),
			DefaultPlaceholderPlugin,
			BeetDefaultPlugins,
			BeetDebugPlugin,
			BeetSimPlugin,
		))
		.add_systems(
			Startup,
			(camera, agent, alcohol, kids_crying, short_stroll),
		)
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

fn orbital_transform(index: usize, total: usize) -> Transform {
	let angle = TAU / total as f32 * index as f32;
	let pos = Vec3::new(f32::cos(angle), f32::sin(angle), 0.);
	Transform::from_translation(pos * 0.7).with_scale(CHILD_SCALE)
}


const CHILD_SCALE: Vec3 = Vec3 {
	x: 0.5,
	y: 0.5,
	z: 0.5,
};


fn agent(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Agent"),
			Emoji::new("1F600"),
			Transform::from_xyz(0., 1., 0.),
		))
		.with_children(|parent| {
			let total_children = 4;

			let agent = parent.parent_entity();
			parent.spawn((
				Name::new(STRESS),
				orbital_transform(0, total_children),
				stat_map.get_id_by_name(STRESS).unwrap(),
				stat_map.get_default_by_name(STRESS).unwrap(),
			));
			parent.spawn((
				Name::new(SELF_CONTROL),
				orbital_transform(1, total_children),
				stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
				stat_map.get_default_by_name(SELF_CONTROL).unwrap(),
			));
			parent.spawn((
				Name::new("Walk"),
				orbital_transform(2, total_children),
				TargetEntity(agent),
				Walk::default(),
			));

			parent
				.spawn((
					Name::new("Behavior"),
					Emoji::new("1F5FA"),
					orbital_transform(3, total_children),
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
						Name::new("Desire Low Stress"),
						TargetEntity(agent),
						StatScoreProvider::new(
							stat_map.get_id_by_name(STRESS).unwrap(),
						)
						.with_low_desired(), // we want stress to be low
					));
					parent.spawn((
						Name::new("Desire High Self Control"),
						TargetEntity(agent),
						StatScoreProvider::new(
							stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
						),
					));
				});
		});
}


fn kids_crying(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Baby Crying"),
			Emoji::new("1F476"),
			CollectableStat::default(),
			Transform::from_xyz(0., -1., 0.),
		))
		.with_child((
			Name::new(STRESS),
			orbital_transform(0, 2),
			stat_map.get_id_by_name(STRESS).unwrap(),
			StatValue::new(0.1),
		));
}
fn alcohol(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Alcohol"),
			Emoji::new("1F37A"),
			CollectableStat::default(),
			Transform::from_xyz(-3., -1., 0.),
		))
		.with_child((
			Name::new(SELF_CONTROL),
			orbital_transform(0, 2),
			stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
			StatValue::new(-0.1),
		));
}

fn short_stroll(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Short Stroll"),
			Emoji::new("1F6B6"),
			CollectableStat::default(),
			Transform::from_xyz(3., -1., 0.),
		))
		.with_child((
			Name::new(STRESS),
			orbital_transform(0, 2),
			stat_map.get_id_by_name(STRESS).unwrap(),
			StatValue::new(-0.1),
		))
		.with_child((
			Name::new(SELF_CONTROL),
			orbital_transform(1, 2),
			stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
			StatValue::new(0.1),
		));
}
