use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

pub mod frozen_lake_assets {
	pub const TILE: &str = "kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0";
	pub const CHARACTER: &str = "kaykit-minigame/character_dog.gltf.glb#Scene0";
	pub const GOAL: &str = "kaykit-minigame/flag_teamYellow.gltf.glb#Scene0";
	pub const HAZARD: &str = "kaykit-minigame/bomb_teamRed.gltf.glb#Scene0";
}

// pub fn spawn_frozen_lake_session(
// 	mut events: EventReader<StartSession<FrozenLakeEpParams>>,
// 	mut commands: Commands,
// ) {
// 	for event in events.read() {
// 		let FrozenLakeEpParams {
// 			map, grid_to_world, ..
// 		} = &event.params;
//
// spawn_frozen_lake_scene(
// 	&mut commands,
// 	map,
// 	grid_to_world,
// 	(SessionEntity(event.session), DespawnOnSessionEnd),
// )
// 	}
// }


pub fn spawn_frozen_lake_episode(
	mut events: EventReader<StartEpisode<FrozenLakeEpParams>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	for event in events.read() {
		let FrozenLakeEpParams {
			map, grid_to_world, ..
		} = &event.params;

		let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

		let agent_pos = map.agent_position();
		let agent_pos = grid_to_world.world_pos(*agent_pos);

		commands
			.spawn((
				SceneRoot(asset_server.load(frozen_lake_assets::CHARACTER)),
				Transform::from_translation(agent_pos).with_scale(object_scale),
				grid_to_world.clone(),
				RlAgentBundle {
					state: map.agent_position(),
					action: GridDirection::sample(),
					env: QTableEnv::new(map.transition_outcomes()),
					params: event.params.learn_params.clone(),
					session: SessionEntity(event.session),
					despawn: DespawnOnEpisodeEnd,
				},
			))
			.with_children(|parent| {
				let agent = parent.parent_entity();

				parent
					.spawn((
						Name::new("Train Frozen Lake Agent"),
						SequenceFlow::default(),
						Repeat::default(),
					))
					.with_children(|parent| {
						parent.spawn((
							Name::new("Go to grid cell"),
							TranslateGrid::new(Duration::from_millis(100)),
							TargetAgent(agent),
						));
						parent.spawn((
							Name::new("Step environment"),
							TargetAgent(agent),
							StepEnvironment::<FrozenLakeQTableSession>::new(
								event.episode,
							),
						));
					})
					.trigger(OnRun);
			});
	}
}
