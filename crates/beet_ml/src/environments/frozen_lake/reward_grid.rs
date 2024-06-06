use crate::prelude::*;
use bevy::prelude::*;


pub fn reward_grid(
	mut query: Query<(&FrozenLakeMap, &GridPos, &mut Reward), Changed<GridPos>>,
) {
	for (map, pos, mut reward) in query.iter_mut() {
		reward.0 = map.position_to_cell(**pos).reward();
	}
}
