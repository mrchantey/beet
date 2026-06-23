//! Fetch uses sentence similarity to decide what to do next, demonstrating:
//! - Machine Learning (Bert embeddings select the closest item)
//! - Animation (idle / walk Foxie animations)
//! - UI (terminal input drives the behavior)
//!
//! Unlike [`hello_ml`], this example performs a search for any sentence with
//! the [`Collectable`] component.
//!
//! On first run the bert model (~90MB) is downloaded via [`fetch_bytes`]
//! and cached locally; subsequent runs hit the cache. Please wait for the
//! status to change to `Idle` before issuing commands.
use beet::examples::scenes;
use beet::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((BeetPlugins, BeetExamplePlugins))
		.add_systems(
			Startup,
			(
				scenes::ui_terminal_input,
				scenes::lighting_3d,
				scenes::ground_3d,
				setup,
				fetch_scene,
				fetch_npc,
			),
		)
		.run();
}

#[rustfmt::skip]
fn setup(mut ev: MessageWriter<OnLogMessage>) {
	ev.write(OnLogMessage::new("Foxie: woof woof! I can fetch the following items:").with_color(OnLogMessage::GAME_COLOR.into()).and_log());
	ev.write(OnLogMessage::new("       - Red healing potion").with_color(OnLogMessage::GAME_COLOR.into()).and_log());
	ev.write(OnLogMessage::new("       - Gold coin").with_color(OnLogMessage::GAME_COLOR.into()).and_log());
	ev.write(OnLogMessage::new("       - Silver sword").with_color(OnLogMessage::GAME_COLOR.into()).and_log());
	ev.write(OnLogMessage::new("       - Tasty cheese").with_color(OnLogMessage::GAME_COLOR.into()).and_log());
}

pub fn fetch_npc(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut anim_graphs: ResMut<Assets<AnimationGraph>>,
) {
	let bert = asset_server.load::<Bert>("ml/default-bert.ron");
	let foxie = asset_server.load("misc/fox.glb#Scene0");

	let Foxie {
		graph_handle,
		idle_index,
		idle_clip: _,
		walk_index,
		walk_clip: _,
	} = Foxie::new(&asset_server, &mut anim_graphs);

	commands.spawn((
		Name::new("Fox"),
		Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(0.01)),
		WorldAssetRoot(foxie.clone()),
		graph_handle,
		AnimationTransitions::new(),
		RotateToVelocity3d::default(),
		ForceBundle::default(),
		SteerBundle {
			max_force: MaxForce(0.05),
			max_speed: MaxSpeed(2.),
			..default()
		},
		OnSpawn::observe(
			|ev: On<Insert, SteerTarget>,
			 steer_targets: Query<&SteerTarget>,
			 sentences: Query<&Sentence>,
			 mut log: MessageWriter<OnLogMessage>| {
				if let Ok(SteerTarget::Entity(steer_target)) =
					steer_targets.get(ev.event_target())
				{
					if let Ok(sentence) = sentences.get(*steer_target) {
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
			},
		),
		children![
			// Plays idle once the [`AnimationPlayer`] is ready so the fox
			// has a resting pose before the user types a command.
			(
				Name::new("Initial Idle"),
				TriggerOnAnimationReady::run(),
				PlayAnimation::new(idle_index).repeat_forever(),
			),
			// User-driven fetch behavior: a sentence is converted to a
			// [`SteerTarget`], the fox walks to it, then returns to idle.
			(
				Name::new("Fetch Behavior"),
				TriggerWithUserSentence,
				Sequence::new(),
				children![
					(
						Name::new("Apply Sentence Steer Target"),
						OnSpawn::new(move |entity| {
							let id = entity.id();
							entity.world_scope(move |world| {
								let parent = world
									.entity(id)
									.get::<ChildOf>()
									.unwrap()
									.parent();
								world.entity_mut(id).insert(
									SentenceSteerTarget::<Collectable>::new(
										bert,
										TargetEntity::Other(parent),
									),
								);
							})
						}),
					),
					(Name::new("Fetch"), Sequence::new(), children![
						(
							Name::new("Play Walk"),
							PlayAnimation::new(walk_index).repeat_forever(),
						),
						(
							Name::new("Seek to Arrive"),
							Seek::default(),
							EndOnArrive::new(1.),
						),
					],),
					(Name::new("Return to Idle"), Sequence::new(), children![
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
					],),
				],
			),
		],
	));
}

pub fn fetch_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
	const ITEM_OFFSET: f32 = 2.;

	// camera
	commands.spawn((
		CameraDistance {
			width: ITEM_OFFSET * 1.4,
			offset: Vec3::new(0., 1.6, ITEM_OFFSET),
		},
		Camera3d::default(),
	));

	// items
	let scale = Vec3::splat(0.5);
	commands.spawn((
		Name::new("Potion"),
		Sentence::new("red healing potion"),
		Collectable,
		Transform::from_xyz(ITEM_OFFSET, 0., ITEM_OFFSET),
		children![(
			Transform::from_xyz(0., 0., 0.).with_scale(scale),
			WorldAssetRoot(asset_server.load("kaykit/potion.glb#Scene0")),
		)],
	));
	commands.spawn((
		Name::new("Coin"),
		Sentence::new("gold coin"),
		Collectable,
		Transform::from_xyz(ITEM_OFFSET, 0., -ITEM_OFFSET),
		children![(
			Transform::from_xyz(0., 0.2, 0.).with_scale(scale),
			WorldAssetRoot(asset_server.load("kaykit/coin.glb#Scene0")),
		)],
	));
	commands.spawn((
		Name::new("Sword"),
		Sentence::new("silver sword"),
		Collectable,
		Transform::from_xyz(-ITEM_OFFSET, 0., ITEM_OFFSET),
		children![(
			Transform::from_xyz(0., 0.15, 0.).with_scale(scale),
			WorldAssetRoot(asset_server.load("kaykit/sword.glb#Scene0")),
		)],
	));
	commands.spawn((
		Name::new("Cheese"),
		Sentence::new("tasty cheese"),
		Collectable,
		Transform::from_xyz(-ITEM_OFFSET, 0., -ITEM_OFFSET),
		children![(
			Transform::from_xyz(0., 0., 0.).with_scale(scale),
			WorldAssetRoot(asset_server.load("kaykit/cheese.glb#Scene0")),
		)],
	));
}
