use crate::prelude::*;
use bevy::prelude::*;


#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct ValencyRender;

pub fn render_valency(
	mut commands: Commands,
	stat_map: Res<StatMap>,
	query: Populated<
		(Entity, &StatId, &StatValue),
		(Changed<StatValue>, With<StatProvider>),
	>,
	children: Query<&Children>,
	existing: Query<&ValencyRender>,
) {
	for (entity, stat_id, value) in query.iter() {
		// 1. remove existing valency renders
		for child in children.iter_descendants(entity) {
			if existing.get(child).is_ok() {
				commands.entity(child).despawn();
			}
		}
		let emoji = match value.is_sign_positive() {
			true => "2795",  // ➕
			false => "2796", // ➖
		};
		let range = *stat_map.get(stat_id).unwrap().total_range();
		let frac = value.0.abs() / range;

		let num_emojis = match frac {
			f if f < 0.33 => 1,
			f if f < 0.66 => 2,
			_ => 3,
		};

		commands.entity(entity).with_children(|parent| {
			for i in 0..num_emojis {
				parent.spawn((Emoji::new(emoji), orbital_child(i, num_emojis)));
			}
		});
	}
}
