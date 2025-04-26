//! Fetch is an example that uses sentence similarity to decide what to do next, demonstrating the following behaviors:
//! - Machine Learning
//! - Animation
//! - UI
//!
//! Unlike [`hello_ml`], this example performs a search for any sentence with
//! the [`Collectable`] component.
//!
//! Please wait for the status to change to `Idle` before issuing commands.
//!
use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins((
			running_beet_example_plugin, 
			plugin_ml
		))
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
fn setup(mut ev: EventWriter<OnLogMessage>) {
	ev.write(OnLogMessage::new("Foxie: woof woof! I can fetch the following items:",OnLogMessage::GAME_COLOR).and_log());
	ev.write(OnLogMessage::new("       - Red healing potion",OnLogMessage::GAME_COLOR).and_log());
	ev.write(OnLogMessage::new("       - Gold coin",OnLogMessage::GAME_COLOR).and_log());
	ev.write(OnLogMessage::new("       - Silver sword",OnLogMessage::GAME_COLOR).and_log());
	ev.write(OnLogMessage::new("       - Tasty cheese",OnLogMessage::GAME_COLOR).and_log());
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

	commands
		.spawn((
			Name::new("Fox"),
			Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(0.01)),
			SceneRoot(foxie.clone()),
			graph_handle,
			AnimationTransitions::new(),
			RotateToVelocity3d::default(),
			ForceBundle::default(),
			SteerBundle {
				max_force: MaxForce(0.05),
				max_speed: MaxSpeed(2.),
				..default()
			},
		))
		.observe(
			|ev: Trigger<OnInsert, SteerTarget>,
			 steer_targets: Query<&SteerTarget>,
			 sentences: Query<&Sentence>,
			 mut log: EventWriter<OnLogMessage>| {
				if let Ok(SteerTarget::Entity(steer_target)) =
					steer_targets.get(ev.target())
				{
					if let Ok(sentence) = sentences.get(*steer_target) {
						log.write(
							OnLogMessage::new(
								format!(
									"Foxie: woof woof! fetching {}",
									sentence.0
								),
								OnLogMessage::GAME_COLOR,
							)
							.and_log(),
						);
					}
				}
			},
		)
		.with_children(|parent| {
			let origin = parent.parent_entity();
			parent
				.spawn((
					Name::new("Fetch Behavior"),
					RunWithUserSentence::new(OnRunAction::new(
						Entity::PLACEHOLDER,
						origin,
						(),
					)),
					Sequence::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Apply Sentence Steer Target"),
						SentenceSteerTarget::<Collectable>::new(
							TargetEntity::Other(parent.parent_entity()),
						),
						HandleWrapper(bert),
						ReturnWith(RunResult::Success),
					));
				})
				.with_child((
					Name::new("Fetch"),
					SteerTargetScoreProvider {
						min_radius: 1.,
						max_radius: 10.,
					},
					Seek::default(),
					PlayAnimation::new(walk_index).repeat_forever(),
					Insert::<OnRun, _>::new_with_target(
						Velocity::default(),
						TargetEntity::Origin,
					),
					EndOnArrive::new(1.),
				))
				.with_child((
					Name::new("Idle"),
					RunOnAnimationReady::default(),
					Remove::<OnRun, Velocity>::new_with_target(
						TargetEntity::Origin,
					),
					PlayAnimation::new(idle_index).repeat_forever(),
				));
		});
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
	commands
		.spawn((
			Name::new("Potion"),
			Sentence::new("red healing potion"),
			Collectable,
			Transform::from_xyz(ITEM_OFFSET, 0., ITEM_OFFSET),
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0., 0.).with_scale(scale),
				SceneRoot(asset_server.load("kaykit/potion.glb#Scene0")),
			));
		});
	commands
		.spawn((
			Name::new("Coin"),
			Sentence::new("gold coin"),
			Collectable,
			Transform::from_xyz(ITEM_OFFSET, 0., -ITEM_OFFSET),
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0.2, 0.).with_scale(scale),
				SceneRoot(asset_server.load("kaykit/coin.glb#Scene0")),
			));
		});
	commands
		.spawn((
			Name::new("Sword"),
			Sentence::new("silver sword"),
			Collectable,
			Transform::from_xyz(-ITEM_OFFSET, 0., ITEM_OFFSET),
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0.15, 0.).with_scale(scale),
				SceneRoot(asset_server.load("kaykit/sword.glb#Scene0")),
			));
		});
	commands
		.spawn((
			Name::new("Cheese"),
			Sentence::new("tasty cheese"),
			Collectable,
			Transform::from_xyz(-ITEM_OFFSET, 0., -ITEM_OFFSET),
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0., 0.).with_scale(scale),
				SceneRoot(asset_server.load("kaykit/cheese.glb#Scene0")),
			));
		});
}
