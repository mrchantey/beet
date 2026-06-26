//! The data form of the frozen-lake demos: a `<FrozenLake/>` tile grid plus either
//! a `<FrozenLakeRunAgent/>` (greedy inference over a trained Q-table) or a
//! `<FrozenLakeTrainSession/>` (an [`RlSession`] that trains one). Mirrors the
//! imperative `frozen_lake_scene` / `frozen_lake_run` / `frozen_lake_train` setups,
//! so a scene `.bsx` names these instead of a Rust `Startup` system.
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// World-space width of the 4x4 grid, shared by every frozen-lake template so the
/// grid, agent and session all map cells to the same world positions.
pub const FROZEN_LAKE_SCENE_SCALE: f32 = 1.;

/// The 4x4 frozen-lake tile grid with its hazards and goal, the data form of
/// `frozen_lake_scene`. Markup has no loop, so the tile + hazard + goal entities are
/// spawned here in Rust (the spawn-N pattern, like [`Flock`](crate::prelude::Flock));
/// they sit at world positions, so they are top-level rather than children of the
/// inert template host. The camera lives in the `.bsx` (a `<Camera3d>` with a
/// [`CameraDistance`]), so this only lays out the board.
#[template(system)]
pub fn FrozenLake(
	mut assets: BuildAssets,
	mut commands: Commands,
) -> impl Bundle {
	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world =
		GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE);

	let tile_scale = Vec3::splat(grid_to_world.cell_width);
	for x in 0..map.num_cols() {
		for y in 0..map.num_rows() {
			let mut pos = grid_to_world.world_pos(UVec2::new(x, y));
			pos.y -= grid_to_world.cell_width;
			commands.spawn((
				Transform::from_translation(pos).with_scale(tile_scale),
				WorldAssetRoot(
					assets.load::<WorldAsset>(frozen_lake_assets::TILE),
				),
			));
		}
	}

	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);
	for (index, cell) in map.cells().iter().enumerate() {
		let grid_pos = map.index_to_position(index);
		let mut pos = grid_to_world.world_pos(grid_pos);
		match cell {
			FrozenLakeCell::Hole => {
				pos.y += grid_to_world.cell_width * 0.25; // this asset is a bit too low
				commands.spawn((
					Transform::from_translation(pos).with_scale(object_scale),
					WorldAssetRoot(
						assets.load::<WorldAsset>(frozen_lake_assets::HAZARD),
					),
				));
			}
			FrozenLakeCell::Goal => {
				commands.spawn((
					WorldAssetRoot(
						assets.load::<WorldAsset>(frozen_lake_assets::GOAL),
					),
					Transform::from_translation(pos).with_scale(object_scale),
				));
			}
			FrozenLakeCell::Ice => { /* already spawned on the grid */ }
			FrozenLakeCell::Agent => { /* spawns with the agent template */ }
		}
	}
}

/// The greedy inference agent, the data form of `frozen_lake_run`'s `setup`: loads
/// the pre-trained Q-table and steers the character across the grid. The qtable
/// handle is minted from the [`AssetServer`] here because a handle is not a markup
/// value; the action tree (a [`Repeat`] driving a [`Sequence`] of [`ReadQPolicy`]
/// then [`TranslateGrid`]) is built as Rust children since [`CallOnSpawn`] and
/// [`ReadQPolicy`] are generic, not markup spreads.
///
/// Run `frozen_lake_train` first to generate `assets/ml/frozen_lake_qtable.ron`.
#[template(system)]
pub fn FrozenLakeRunAgent(
	mut rng: ResMut<RandomSource>,
	mut assets: BuildAssets,
	mut commands: Commands,
) -> impl Bundle {
	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world =
		GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE);

	let agent_grid_pos = map.agent_position();
	let agent_pos = grid_to_world.world_pos(*agent_grid_pos);
	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

	let qtable =
		assets.load::<FrozenLakeQTable>("ml/frozen_lake_qtable.ron");

	// Spawned top-level (not as a child of the scene host) so the action tree
	// resolves its agent to this entity — its root ancestor — rather than the
	// transformless scene root, which lacks the `GridPos`/`GridToWorld` the
	// `ReadQPolicy`/`TranslateGrid` actions query (the original spawned it top-level).
	//
	// Action tree (mirrors `spawn_frozen_lake_episode`):
	//   Repeat
	//     Sequence "Run Frozen Lake Agent"
	//       ReadQPolicy   — greedy action lookup from the trained Q-table
	//       TranslateGrid — animate the move
	commands.spawn((
		Name::new("Inference Agent"),
		WorldAssetRoot(assets.load::<WorldAsset>(frozen_lake_assets::CHARACTER)),
		Transform::from_translation(agent_pos).with_scale(object_scale),
		grid_to_world,
		agent_grid_pos,
		GridDirection::sample(&mut rng.0),
		Repeat::<()>::default(),
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Name::new("Run Frozen Lake Agent"),
			Sequence::<(), ()>::default(),
			children![
				(
					Name::new("Get next action"),
					ReadQPolicy::<FrozenLakeQTable>::new(qtable),
				),
				(
					Name::new("Perform action"),
					TranslateGrid::new(Duration::from_secs(1)),
				)
			]
		)],
	));
}

/// The training session, the data form of `frozen_lake_train`'s `setup`: spawns an
/// [`RlSession`] plus an empty [`FrozenLakeQTable`] for it to fill. The per-episode
/// agent is spawned by `spawn_frozen_lake_episode` once the session emits its first
/// `StartEpisode`, so this only seeds the session.
#[template(system)]
pub fn FrozenLakeTrainSession() -> impl Bundle {
	let map = FrozenLakeMap::default_four_by_four();
	let params = FrozenLakeEpParams {
		learn_params: default(),
		grid_to_world: GridToWorld::from_frozen_lake_map(
			&map,
			FROZEN_LAKE_SCENE_SCALE,
		),
		map,
	};
	(RlSession::new(params), FrozenLakeQTable::default())
}
