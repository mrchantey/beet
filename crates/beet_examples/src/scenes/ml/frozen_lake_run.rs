use super::*;
use crate::prelude::*;
use beet_flow::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

pub fn frozen_lake_run(mut commands: Commands) {
	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world =
		GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE);

	// agent
	let agent_grid_pos = map.agent_position();
	let agent_pos = grid_to_world.world_pos(*agent_grid_pos);
	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

	commands
		.spawn((
			Name::new("Inference Agent"),
			BundlePlaceholder::Scene(frozen_lake_assets::CHARACTER.into()),
			Transform::from_translation(agent_pos).with_scale(object_scale),
			grid_to_world.clone(),
			agent_grid_pos,
			GridDirection::sample(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			parent
				.spawn((
					Name::new("Run Frozen Lake Agent"),
					RunOnAppReady::default(),
					SequenceFlow,
					Repeat::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Get next action"),
						TargetAgent(agent),
						AssetLoadBlockAppReady,
						AssetPlaceholder::<FrozenLakeQTable>::new(
							"ml/frozen_lake_qtable.ron",
						),
						ReadQPolicy::<FrozenLakeQTable>::default(),
					));
					parent.spawn((
						Name::new("Perform action"),
						ContinueRun::default(),
						TranslateGrid::new(Duration::from_secs(1)),
						TargetAgent(agent),
						RunTimer::default(),
					));
				});
		});
}
