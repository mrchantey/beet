use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::time::Duration;

/// Asset paths for the demo's 3D scene.
pub mod frozen_lake_assets {
	/// Tile (floor) asset.
	pub const TILE: &str = "kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0";
	/// Character (agent) asset.
	pub const CHARACTER: &str = "kaykit-minigame/character_dog.gltf.glb#Scene0";
	/// Goal-flag asset.
	pub const GOAL: &str = "kaykit-minigame/flag_teamYellow.gltf.glb#Scene0";
	/// Hole / hazard asset.
	pub const HAZARD: &str = "kaykit-minigame/bomb_teamRed.gltf.glb#Scene0";
}

/// On each [`StartEpisode`], spawns the agent entity plus its action
/// tree: a top-level [`Sequence`] containing a [`Repeat`] that alternates
/// [`TranslateGrid`] (walk to next cell) with
/// [`StepEnvironment`] (record reward, choose next action).
///
/// The agent is kicked off via [`CallOnSpawn`], so the action tree starts
/// running as soon as the entity is fully spawned.
pub fn spawn_frozen_lake_episode(
	mut events: MessageReader<StartEpisode<FrozenLakeEpParams>>,
	mut rng: ResMut<RandomSource>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	for event in events.read() {
		let FrozenLakeEpParams {
			map, grid_to_world, ..
		} = &event.params;

		let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);
		let agent_pos = grid_to_world.world_pos(*map.agent_position());

		// Action tree:
		//   Repeat (driven by CallOnSpawn)
		//     Sequence "Train Frozen Lake Agent"
		//       TranslateGrid
		//       StepEnvironment
		//
		// In beet_action, Repeat calls its *single* first child every iteration —
		// the old beet_flow trick of stacking Repeat + Sequence on one entity no
		// longer works, so Sequence becomes Repeat's only child.
		commands.spawn((
			WorldAssetRoot(asset_server.load(frozen_lake_assets::CHARACTER)),
			Transform::from_translation(agent_pos).with_scale(object_scale),
			grid_to_world.clone(),
			RlAgentBundle {
				state: map.agent_position(),
				action: GridDirection::sample(&mut rng.0),
				env: QTableEnv::new(map.transition_outcomes()),
				params: event.params.learn_params.clone(),
				session: SessionEntity(event.session),
				despawn: DespawnOnEpisodeEnd,
			},
			Name::new("Frozen Lake Agent"),
			Repeat::<()>::default(),
			CallOnSpawn::<(), Outcome>::default(),
			children![(
				Name::new("Train Frozen Lake Agent"),
				Sequence::<(), ()>::default(),
				children![
					(
						Name::new("Go to grid cell"),
						TranslateGrid::new(Duration::from_millis(100)),
					),
					(
						Name::new("Step environment"),
						StepEnvironment::<FrozenLakeQTableSession>::new(
							event.episode,
						),
					),
				],
			)],
		));
	}
}
