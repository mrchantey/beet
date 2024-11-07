use beet_flow::prelude::*;
use beet_sim::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;

fn main() {
	let mut stat_map = StatMap::default();
	stat_map.add_stat(StatDescriptor {
		name: "Health".to_string(),
		description: "The health of the entity".to_string(),
		emoji_hexcode: "2764".to_string(),
		global_range: 0.0..1.,
	});
	stat_map.add_stat(StatDescriptor {
		name: "Energy".to_string(),
		description: "The energy of the entity".to_string(),
		emoji_hexcode: "26A1".to_string(),
		global_range: 0.0..1.,
	});
	App::new()
		.add_plugins((
			BeetmashDefaultPlugins::with_native_asset_path("../../assets"),
			DefaultPlaceholderPlugin,
			BeetSimPlugin,
		))
		.add_systems(Startup, (camera, agent, cupcake, gym))
		.insert_resource(stat_map)
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
			let agent = parent.parent_entity();
			parent.spawn((
				Name::new("Health"),
				Transform::from_translation(orbital_pos(0, 3))
					.with_scale(CHILD_SCALE),
				Stat::new(100., stat_map.get_by_name("Health").unwrap()),
			));
			parent.spawn((
				Name::new("Energy"),
				Transform::from_translation(orbital_pos(1, 3))
					.with_scale(CHILD_SCALE),
				Stat::new(100., stat_map.get_by_name("Energy").unwrap()),
			));
			parent.spawn((
				Name::new("Walk"),
				Transform::from_translation(orbital_pos(2, 3))
					.with_scale(CHILD_SCALE),
				TargetEntity(agent),
				Walk::default(),
			));
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
			Stat::new(-0.1, stat_map.get_by_name("Health").unwrap()),
		))
		.with_child((
			Name::new("Energy"),
			Transform::from_translation(orbital_pos(1, 2))
				.with_scale(CHILD_SCALE),
			Stat::new(0.1, stat_map.get_by_name("Energy").unwrap()),
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
			Stat::new(0.1, stat_map.get_by_name("Health").unwrap()),
		))
		.with_child((
			Name::new("Energy"),
			Transform::from_translation(orbital_pos(1, 2))
				.with_scale(CHILD_SCALE),
			Stat::new(-0.1, stat_map.get_by_name("Energy").unwrap()),
		));
}
