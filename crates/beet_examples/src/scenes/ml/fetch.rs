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
/// scaled glb model child. The camera + terminal live in the `.bsx`.
#[template(system)]
pub fn FetchItems(
	asset_server: Res<AssetServer>,
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
				WorldAssetRoot(asset_server.load(src)),
			)],
		));
	}
}

/// The fetch NPC, the data form of `fetch_npc`: the animated fox plus a
/// user-driven behaviour tree that converts the typed prompt into a
/// [`SteerTarget`], walks the fox there, then returns it to idle.
///
/// Spawned top-level (not as the returned bundle) so the action tree resolves its
/// agent to this entity — its root ancestor, which carries the steering
/// components — rather than the transformless scene root (the frozen-lake lesson).
/// The `Handle<Bert>` is minted from the [`AssetServer`] here because a handle is
/// not a markup value, and [`SentenceSteerTarget`] is a generic action, not a
/// markup spread, so the whole tree is built in Rust `children!`.
///
/// Behaviour tree (mirrors `fetch_npc`):
///   Initial Idle   — play idle once the [`AnimationPlayer`] is ready
///   Fetch Behavior — on each user sentence:
///     Sequence
///       Apply Sentence Steer Target — closest [`Collectable`] -> [`SteerTarget`]
///       Fetch  — play walk, then [`Seek`] until [`EndOnArrive`]
///       Return to Idle — stop moving, play idle
#[template(system)]
pub fn FetchFox(
	asset_server: Res<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
	mut commands: Commands,
) -> impl Bundle {
	let bert = asset_server.load::<Bert>("ml/default-bert.ron");

	// build the idle (Animation0) + walk (Animation1) graph, keeping the node
	// indices the tree's `PlayAnimation` actions play. Built here (not via the
	// `<Foxie>` template) so the fox is spawned top-level with its tree attached.
	let mut graph = AnimationGraph::new();
	let root = graph.root;
	let idle_index = graph.add_clip(
		asset_server.load::<AnimationClip>("misc/fox.glb#Animation0"),
		1.0,
		root,
	);
	let walk_index = graph.add_clip(
		asset_server.load::<AnimationClip>("misc/fox.glb#Animation1"),
		1.0,
		root,
	);

	let fox = commands
		.spawn((
			Name::new("Fox"),
			Transform::from_scale(Vec3::splat(0.01)),
			WorldAssetRoot(asset_server.load("misc/fox.glb#Scene0")),
			AnimationGraphHandle(graphs.add(graph)),
			AnimationTransitions::new(),
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
		))
		.id();

	// the `TriggerWithUserSentence` node holds the user's typed prompt, so the
	// steer-target search reads its `Sentence` (`TargetEntity::Other`).
	let fetch_behavior = commands
		.spawn((
			Name::new("Fetch Behavior"),
			ChildOf(fox),
			TriggerWithUserSentence,
			Sequence::new(),
		))
		.id();

	commands.entity(fox).with_children(|fox| {
		// plays idle once the `AnimationPlayer` is ready so the fox has a resting
		// pose before the user types a command.
		fox.spawn((
			Name::new("Initial Idle"),
			TriggerOnAnimationReady::run(),
			PlayAnimation::new(idle_index).repeat_forever(),
		));
	});

	commands.entity(fetch_behavior).with_children(|seq| {
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
				PlayAnimation::new(walk_index).repeat_forever(),
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
				PlayAnimation::new(idle_index).repeat_forever(),
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
