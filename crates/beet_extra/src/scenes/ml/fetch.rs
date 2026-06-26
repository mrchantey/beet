//! The data form of the `fetch` demo: a `<FetchItems/>` set of [`Collectable`]
//! items plus a `<FetchFox/>` NPC that walks to the item whose [`Sentence`] best
//! matches the user's typed prompt. Mirrors the imperative `fetch_scene` /
//! `fetch_npc` setups, so a scene `.bsx` names these instead of a Rust `Startup`
//! system. The behaviour tree references a `Handle<Bert>` and generic actions
//! ([`SentenceSteerTarget`]) that are not markup values, so the fox is built in
//! Rust rather than authored as `.bsx` children.
use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// World-space offset of each item from the origin, shared by [`FetchItems`] and
/// the camera so the four collectibles frame the fox at the centre.
pub const FETCH_ITEM_OFFSET: f32 = 2.;

/// The four fetchable [`Collectable`] items (potion / coin / sword / cheese), the
/// data form of `fetch_scene`'s item spawns. Markup has no loop, so the items are
/// spawned here in Rust (the spawn-N pattern, like [`Flock`](crate::prelude::Flock));
/// they sit at world positions, so they are top-level rather than children of the
/// inert template host. Each carries a [`Sentence`] the fox matches against, and a
/// scaled glb model child loaded through the deferred [`BuildAssets`] path so the
/// scene's `LoadTemplate` waits for every model. The camera + terminal live in the
/// `.bsx`.
#[template(system)]
pub fn FetchItems(
	mut assets: BuildAssets,
	mut commands: Commands,
) -> impl Bundle {
	let scale = Vec3::splat(0.5);
	let offset = FETCH_ITEM_OFFSET;
	// (name, sentence, glb path, +y model nudge, x, z)
	let items = [
		(
			"Potion",
			"red healing potion",
			"kaykit/potion.glb#Scene0",
			0.0,
			offset,
			offset,
		),
		(
			"Coin",
			"gold coin",
			"kaykit/coin.glb#Scene0",
			0.2,
			offset,
			-offset,
		),
		(
			"Sword",
			"silver sword",
			"kaykit/sword.glb#Scene0",
			0.15,
			-offset,
			offset,
		),
		(
			"Cheese",
			"tasty cheese",
			"kaykit/cheese.glb#Scene0",
			0.0,
			-offset,
			-offset,
		),
	];
	for (name, sentence, src, model_y, x, z) in items {
		commands.spawn((
			Name::new(name),
			Sentence::new(sentence),
			Collectable,
			Transform::from_xyz(x, 0., z),
			// the item carries a transform already, so `ensure_spatial_roots` skips it;
			// give it `Visibility` explicitly so its glb model child inherits one.
			Visibility::default(),
			children![(
				Transform::from_xyz(0., model_y, 0.).with_scale(scale),
				WorldAssetRoot(assets.load::<WorldAsset>(src)),
			)],
		));
	}
}

/// The fetch NPC, the data form of `fetch_npc`: the animated fox plus a
/// user-driven behaviour tree that converts the typed prompt into a
/// [`SteerTarget`], walks the fox there, then returns it to idle.
///
/// The `<FetchFox/>` tag entity *is* the fox: the template returns the fox bundle,
/// so the fox is hosted under the scene's build root (eg `<Scene3d>`) and receives
/// `LoadTemplate` once its deferred assets load. The behaviour tree is spawned as
/// the fox's children via an [`OnSpawn`] hook (which yields the fox entity), each
/// tree root wired with `ActionOf(fox)` so the actions resolve their agent to the
/// fox (its [`AnimationGraphClips`], [`SteerBundle`] and [`SteerTarget`] live here).
/// The graph is built through the deferred [`BuildAssets`] path so `LoadTemplate`
/// waits for both fox clips and the glb scene; the `Handle<Bert>` is minted the
/// same way because a handle is not a markup value and [`SentenceSteerTarget`] is a
/// generic action, not a markup spread, so the whole tree is built in Rust.
///
/// Behaviour tree (mirrors `fetch_npc`):
///   Initial Idle   — play idle once the fox's assets load ([`RunOnLoad`])
///   Fetch Behavior — on each user sentence:
///     Sequence
///       Apply Sentence Steer Target — closest [`Collectable`] -> [`SteerTarget`]
///       Fetch  — play walk, then [`Seek`] until [`EndOnArrive`]
///       Return to Idle — stop moving, play idle
#[template(system)]
pub fn FetchFox(
	mut graphs: ResMut<Assets<AnimationGraph>>,
	mut assets: BuildAssets,
) -> impl Bundle {
	let bert = assets.load::<Bert>("ml/default-bert.ron");
	// the idle (Animation0) + walk (Animation1) graph; the actions resolve their
	// clip paths against the `AnimationGraphClips` it puts on the fox.
	let clips = vec![
		"misc/fox.glb#Animation0".to_string(),
		"misc/fox.glb#Animation1".to_string(),
	];
	(
		Name::new("Fox"),
		Transform::from_scale(Vec3::splat(0.01)),
		WorldAssetRoot(assets.load::<WorldAsset>("misc/fox.glb#Scene0")),
		build_animation_graph(&clips, &mut graphs, &mut assets),
		RotateToVelocity3d::default(),
		ForceBundle::default(),
		SteerBundle {
			max_force: MaxForce(0.05),
			max_speed: MaxSpeed(2.),
			..default()
		},
		// announce the chosen item to the terminal when the fetch action sets
		// the fox's `SteerTarget` (scoped to the fox via `observe`).
		OnSpawn::observe(announce_fetch),
		// build the behaviour tree as the fox's children, wiring each tree root's
		// agent to the fox so its actions resolve here (`AnimationGraphClips`,
		// `SteerBundle`, `SteerTarget`).
		OnSpawn::new(move |fox| {
			let fox_id = fox.id();
			fox.world_scope(|world| spawn_fetch_tree(world, fox_id, bert));
		}),
	)
}

/// Spawn the fetch behaviour tree as children of `fox`, each root acting on the
/// fox via [`ActionOf`]. Split out of [`FetchFox`]'s [`OnSpawn`] hook so the tree
/// structure reads top-down.
fn spawn_fetch_tree(world: &mut World, fox: Entity, bert: Handle<Bert>) {
	// plays idle once the fox's assets load (`RunOnLoad`), so the fox has a resting
	// pose before the user types a command.
	world.spawn((
		Name::new("Initial Idle"),
		ChildOf(fox),
		ActionOf(fox),
		RunOnLoad,
		PlayAnimation::new("misc/fox.glb#Animation0").repeat_forever(),
	));

	// the `TriggerWithUserSentence` node holds the user's typed prompt, so spawn it
	// first to get its id, then point the steer-target search at its `Sentence`.
	let fetch_behavior = world
		.spawn((
			Name::new("Fetch Behavior"),
			ChildOf(fox),
			ActionOf(fox),
			TriggerWithUserSentence,
			Sequence::new(),
		))
		.id();
	world.entity_mut(fetch_behavior).with_children(|seq| {
		seq.spawn((
			Name::new("Apply Sentence Steer Target"),
			SentenceSteerTarget::<Collectable>::new(
				bert,
				TargetEntity::Other(fetch_behavior),
			),
		));
		seq.spawn((Name::new("Fetch"), Sequence::new(), children![
			(
				Name::new("Play Walk"),
				PlayAnimation::new("misc/fox.glb#Animation1").repeat_forever(),
			),
			(
				Name::new("Seek to Arrive"),
				Seek::default(),
				EndOnArrive::new(1.),
			),
		]));
		seq.spawn((Name::new("Return to Idle"), Sequence::new(), children![
			(
				Name::new("Stop Moving"),
				InsertOn::new_with_target(
					Velocity::default(),
					TargetEntity::Agent,
				),
			),
			(
				Name::new("Play Idle"),
				PlayAnimation::new("misc/fox.glb#Animation0").repeat_forever(),
			),
		]));
	});
}

/// Logs the [`Sentence`] of the item the fox is fetching once the fetch action sets
/// its [`SteerTarget`], the data form of `fetch_npc`'s spawn-time observer.
fn announce_fetch(
	ev: On<Insert, SteerTarget>,
	steer_targets: Query<&SteerTarget>,
	sentences: Query<&Sentence>,
	mut log: MessageWriter<OnLogMessage>,
) {
	if let Ok(SteerTarget::Entity(steer_target)) =
		steer_targets.get(ev.event_target())
		&& let Ok(sentence) = sentences.get(*steer_target)
	{
		log.write(
			OnLogMessage::new(format!(
				"Foxie: woof woof! fetching {}",
				sentence.0
			))
			.with_color(OnLogMessage::GAME_COLOR.into())
			.and_log(),
		);
	}
}
