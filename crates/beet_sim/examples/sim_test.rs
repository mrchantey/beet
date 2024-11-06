use beet_flow::prelude::*;
use beet_sim::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::any::TypeId;
use std::f32::consts::TAU;

fn main() {
	App::new()
		.add_plugins((
			BeetmashDefaultPlugins::with_native_asset_path("../../assets"),
			DefaultPlaceholderPlugin,
			BeetSimPlugin,
		))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		Transform::from_xyz(0., 0., 5.),
	));


	let mut stat_map = StatMap::default();
	let health_stat_id = stat_map.add_stat(StatDescriptor {
		name: "Health".to_string(),
		description: "The health of the entity".to_string(),
		emoji_hexcode: "2764".to_string(),
		type_id: TypeId::of::<f32>(),
	});
	let energy_stat_id = stat_map.add_stat(StatDescriptor {
		name: "Energy".to_string(),
		description: "The energy of the entity".to_string(),
		emoji_hexcode: "26A1".to_string(),
		type_id: TypeId::of::<f32>(),
	});
	commands.insert_resource(stat_map);

	commands
		.spawn((Name::new("Agent"), Emoji::new("1F600")))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			let mut incr = 0;
			let mut next_pos = || {
				let offset = 1.;
				let angle = TAU / 5. * incr as f32;
				let pos =
					Vec3::new(f32::cos(angle), f32::sin(angle), 0.) * offset;
				incr += 1;
				println!("pos: {:?}", pos);
				pos
			};

			parent.spawn((
				Name::new("Health"),
				Transform::from_translation(next_pos()),
				Stat::<f32>::new(100., health_stat_id),
			));
			parent.spawn((
				Name::new("Energy"),
				Transform::from_translation(next_pos()),
				Stat::<f32>::new(100., energy_stat_id),
			));
			parent.spawn((
				Name::new("Walk"),
				Transform::from_translation(next_pos()),
				TargetEntity(agent),
				Walk::default(),
			));
		});
}
