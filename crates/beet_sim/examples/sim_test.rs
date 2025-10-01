use beet_flow::prelude::*;
use beet_sim::prelude::*;
use beet_spatial::prelude::*;
use beet_core::prelude::*;


const STRESS: &str = "Stress";
const SELF_CONTROL: &str = "Self-Control";

fn main() {
	let stat_map = StatMap::default()
		.with_stat(StatDescriptor {
			name: SELF_CONTROL.to_string(),
			description: "Ability to make good decisions".to_string(),
			emoji_hexcode: "1F9D8".to_string(), //🧘
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(1.),
		})
		.with_stat(StatDescriptor {
			name: STRESS.to_string(),
			description: "Current stress level".to_string(),
			emoji_hexcode: "1F92F".to_string(), //🤯
			global_range: StatValue::range(0.0..1.),
			default_value: StatValue(0.),
		});
	App::new()
		.add_plugins((BeetFlowPlugin::default(), BeetSimPlugin))
		.add_systems(
			Startup,
			(camera, agent, alcohol, kids_crying, short_stroll),
		)
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

fn agent(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Agent"),
			Emoji::new("1F600"), //😀
			Transform::from_xyz(0., 1., 0.),
			MaxSpeed::default(),
			Collector,
		))
		.with_children(|parent| {
			let total_children = 4;

			let _agent = parent.target_entity();
			let _stress = parent
				.spawn((
					Name::new(STRESS),
					orbital_child(0, total_children),
					stat_map.get_id_by_name(STRESS).unwrap(),
					stat_map.get_default_by_name(STRESS).unwrap(),
				))
				.id();
			let _self_control = parent
				.spawn((
					Name::new(SELF_CONTROL),
					orbital_child(1, total_children),
					stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
					stat_map.get_default_by_name(SELF_CONTROL).unwrap(),
				))
				.id();
			parent.spawn((
				Name::new("Walk"),
				orbital_child(2, total_children),
				Walk::default(),
			));

			todo!("run on change");
			#[allow(unused)]
			parent
				.spawn((
					Name::new("Behavior"),
					Emoji::new("1F5FA"), //🗺️
					orbital_child(3, total_children),
					TriggerDeferred::default(),
					// RunOnChange::<StatValue>::default()
					// 	.with_source(vec![stress, self_control]),
					HighestScore::default(),
					// RepeatFlow::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						ReturnWith(ScoreValue::NEUTRAL),
					));
					parent.spawn((
						Name::new("Desire Low Stress"),
						stat_map.get_id_by_name(STRESS).unwrap(),
						StatScoreProvider::default(), // we want stress to be low
						StatValueGoal::Low,
						FindStatSteerTarget::default(),
						Seek::default(),
					));
					parent.spawn((
						Name::new("Desire High Self Control"),
						stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
						StatScoreProvider::default(),
						FindStatSteerTarget::default(),
						Seek::default(),
					));
				});
		});
}


fn kids_crying(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Baby Crying"),
			Emoji::new("1F476"), //👶
			Transform::from_xyz(0., -1., 0.),
			MaxSpeed::default(),
			CollectableStat::default(),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Seek Agent"),
				Emoji::new("1F5FA"), //🗺️
				orbital_child(0, 2),
				TriggerDeferred::default(),
				Seek::default(),
				FindSteerTarget::new("Agent", f32::MAX),
			));

			parent.spawn((
				Name::new(STRESS),
				orbital_child(1, 2),
				stat_map.get_id_by_name(STRESS).unwrap(),
				StatValue::new(0.1),
				StatProvider::default(),
			));
		});
}
fn alcohol(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Alcohol"),
			Emoji::new("1F37A"), //🍺
			CollectableStat::default(),
			Transform::from_xyz(-3., -1., 0.),
		))
		.with_child((
			Name::new(STRESS),
			orbital_child(0, 2),
			stat_map.get_id_by_name(STRESS).unwrap(),
			StatValue::new(-0.1),
			StatProvider::default(),
		))
		.with_child((
			Name::new(SELF_CONTROL),
			orbital_child(1, 2),
			stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
			StatValue::new(-0.1),
			StatProvider::default(),
		));
}

fn short_stroll(mut commands: Commands, stat_map: Res<StatMap>) {
	commands
		.spawn((
			Name::new("Short Stroll"),
			Emoji::new("1F6B6"), //🚶
			ZoneStat::default(),
			Transform::from_xyz(3., -1.5, 0.),
		))
		.with_child((
			Name::new(STRESS),
			orbital_child(0, 2),
			stat_map.get_id_by_name(STRESS).unwrap(),
			StatValue::new(-0.1),
			StatProvider::default(),
		))
		.with_child((
			Name::new(SELF_CONTROL),
			orbital_child(1, 2),
			stat_map.get_id_by_name(SELF_CONTROL).unwrap(),
			StatValue::new(0.1),
			StatProvider::default(),
		));
}
