use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::time::Duration;


#[derive(Resource)]
pub struct FrozenLakeAssets {
	pub tile: Handle<Scene>,
	pub character: Handle<Scene>,
	pub goal: Handle<Scene>,
	pub hazard: Handle<Scene>,
}

pub fn init_frozen_lake_assets(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	let tile =
		asset_server.load("kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0");
	let character =
		asset_server.load("kaykit-minigame/character_dog.gltf.glb#Scene0");
	let goal =
		asset_server.load("kaykit-minigame/flag_teamYellow.gltf.glb#Scene0");
	let hazard =
		asset_server.load("kaykit-minigame/bomb_teamRed.gltf.glb#Scene0");

	commands.insert_resource(FrozenLakeAssets {
		tile,
		character,
		goal,
		hazard,
	});
}

pub fn spawn_frozen_lake_session(
	mut events: EventReader<StartSession<FrozenLakeEpParams>>,
	mut commands: Commands,
	assets: Res<FrozenLakeAssets>,
) {
	for event in events.read() {
		let FrozenLakeEpParams {
			map, grid_to_world, ..
		} = &event.params;

		spawn_frozen_lake_scene(
			&mut commands,
			map,
			grid_to_world,
			&assets,
			(SessionEntity(event.session), DespawnOnSessionEnd),
		)
	}
}


pub fn spawn_frozen_lake_episode(
	mut events: EventReader<StartEpisode<FrozenLakeEpParams>>,
	mut commands: Commands,
	assets: Res<FrozenLakeAssets>,
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
				SceneBundle {
					scene: assets.character.clone(),
					transform: Transform::from_translation(agent_pos)
						.with_scale(object_scale),
					..default()
				},
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
					.spawn((Running, SequenceSelector, Repeat::default()))
					.with_children(|parent| {
						parent.spawn((
							TranslateGrid::new(Duration::from_millis(100)),
							TargetAgent(agent),
							RunTimer::default(),
						));
						parent.spawn((
							TargetAgent(agent),
							StepEnvironment::<FrozenLakeQTableSession>::new(
								event.episode,
							),
						));
					});
			});
	}
}
