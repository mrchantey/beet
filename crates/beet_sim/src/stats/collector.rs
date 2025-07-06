use crate::prelude::*;
use bevy::prelude::*;


/// Marker to indicate that an entity is a collector.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Collector;



impl Collector {
	pub fn apply() {}
}




pub fn apply_stats(
	commands: &mut Commands,
	stat_map: &Res<StatMap>,
	collector_entity: Entity,
	collectable_entity: Entity,
	children: &Query<&Children>,
	stats: &mut Query<(&mut StatId, &mut StatValue)>,
) {
	let stats_to_apply = children
		.iter_descendants(collectable_entity)
		.filter_map(|child| {
			stats.get(child).ok().map(|(id, value)| (*id, *value))
		})
		.collect::<Vec<_>>();

	for (stat_id, stat_value) in stats_to_apply {
		if let Ok((collector_stat_id, mut collector_stat_value)) =
			stats.get_mut(collector_entity)
			&& stat_id == *collector_stat_id
		{
			**collector_stat_value += *stat_value;
		} else {
			let stat_entry = stat_map
				.get(&stat_id)
				.expect(format!("StatId not found: {:?}", stat_id).as_str());


			let new_stat = commands
				.spawn((
					Name::new(stat_entry.name.clone()),
					stat_id.clone(),
					stat_value.clone(),
					// orbital_child(0, 0), //TODO reposition all children
				))
				.id();
			commands.entity(collector_entity).add_child(new_stat);
		}
	}
}
