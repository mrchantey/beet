//! Fetch is a combined example demonstrating the following behaviors:
//! - Machine Learning
//! - Animation
//! - UI
//!
//! Please wait for the status to change to `Idle` before issuing commands.
//!
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

const ITEM_OFFSET: f32 = 2.;


fn main() {
	let mut app = App::new();
	app.add_plugins((
		ExamplePlugin3d::default(),
		DefaultBeetPlugins,
		// BeetDebugPlugin::default(),
		DialogPanelPlugin,
		BertPlugin,
		ActionPlugin::<(
			SetTextOnRun<StatusOutput>,
			InsertOnAssetEvent<RunResult, Bert>,
			FindSentenceSteerTarget<Item>,
			RemoveAgentOnRun<Sentence>,
			RemoveAgentOnRun<SteerTarget>,
		)>::default(),
	))
	.add_systems(Startup, (setup_camera, setup_fox, setup_items))
	.add_systems(Update, (set_player_sentence, rotate_items));

	app.run();
}

fn setup_camera(mut commands: Commands) {
	commands.spawn((
		CameraDistance {
			width: ITEM_OFFSET * 1.4,
			offset: Vec3::new(0., 1.6, ITEM_OFFSET),
		},
		Camera3dBundle::default(),
	));
}

#[derive(Component)]
pub struct Player;

fn setup_fox(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut ready_on_load: ResMut<ReadyOnAssetLoad>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) {
	let mut graph = AnimationGraph::new();

	let idle_anim_clip = asset_server.load("Fox.glb#Animation0");
	let idle_anim_index =
		graph.add_clip(idle_anim_clip.clone(), 1.0, graph.root);
	let walk_anim_clip = asset_server.load("Fox.glb#Animation1");
	let walk_anim_index =
		graph.add_clip(walk_anim_clip.clone(), 1.0, graph.root);

	commands
		.spawn((
			Player,
			SceneBundle {
				scene: asset_server.load("Fox.glb#Scene0"),
				transform: Transform::from_xyz(0., 0., 0.)
					.with_scale(Vec3::splat(0.01)),
				..default()
			},
			graphs.add(graph),
			AnimationTransitions::new(),
			RotateToVelocity3d::default(),
			ForceBundle::default(),
			SteerBundle {
				max_force: MaxForce(0.05),
				max_speed: MaxSpeed(2.),
				..default()
			}
			.scaled_to(1.),
			// Uncomment this to have an initial target
			// Sentence::new("tasty"),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			let bert_handle = asset_server.load("default-bert.ron");
			ready_on_load.insert("default-bert.ron".to_string());
			parent
				.spawn((
					Name::new("Fetch Behavior"),
					Running,
					SequenceSelector,
					Repeat,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Await Bert Load"),
						InsertOnAssetEvent::loaded(
							RunResult::Success,
							&bert_handle,
						),
					));
					parent
						.spawn((
							Name::new("Idle Or Fetch"),
							TriggerOnRun(AppReady),
							TargetAgent(agent),
							ScoreSelector::default(),
							// ScoreSelector::consuming(),
							FindSentenceSteerTarget::<Item>::new(bert_handle),
						))
						.with_children(|parent| {
							parent.spawn((
								Name::new("Idle"),
								Score::neutral(),
								TargetAgent(agent),
								SetAgentOnRun(Velocity::default()),
								PlayAnimation::new(idle_anim_index)
									.repeat_forever(),
								RunTimer::default(),
								InsertInDuration::new(
									RunResult::Success,
									Duration::from_secs(1),
								),
								SetTextOnRun::<StatusOutput>::new_with_section(
									"Idle", 1,
								),
							));
							parent
								.spawn((
									Name::new("Fetch"),
									Score::default(),
									TargetAgent(agent),
									ScoreSteerTarget::new(10.),
									PlayAnimation::new(walk_anim_index)
										.repeat_forever(),
									SequenceSelector,
									RemoveAgentOnRun::<Sentence>::default(),
									SetTextOnRun::<StatusOutput>::new_with_section(
										"Fetching",1
									),
								))
								.with_children(|parent| {
									parent.spawn((
										Name::new("Go To Item"),
										TargetAgent(agent),
										Seek,
										SucceedOnArrive::new(1.),
									));
									parent.spawn((
										Name::new("Pick Up Item"),
										TargetAgent(agent),
										// SetAgentOnRun(SteerTarget::Position(
										// 		Vec3::ZERO,
										// 	)),
										RemoveAgentOnRun::<SteerTarget>::default(),
										InsertOnRun(RunResult::Success),
									));
									// parent.spawn((
									// 	Name::new("Return Item To Center"),
									// 	TargetAgent(agent),
									// 	Seek,
									// 	SucceedOnArrive::new(6.),
									// ));
								});
						});
				});
		});
}


#[derive(Clone, Component, Reflect)]
struct Item;

fn setup_items(mut commands: Commands, asset_server: Res<AssetServer>) {
	let scale = Vec3::splat(0.5);
	commands
		.spawn((
			Name::new("Potion"),
			Sentence::new("red healing potion"),
			Item,
			SpatialBundle {
				transform: Transform::from_xyz(ITEM_OFFSET, 0., ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn(SceneBundle {
				scene: asset_server.load("kaykit/potion.glb#Scene0"),
				transform: Transform::from_xyz(0., 0., 0.).with_scale(scale),
				..default()
			});
		});
	commands
		.spawn((
			Name::new("Coin"),
			Sentence::new("gold coin"),
			Item,
			SpatialBundle {
				transform: Transform::from_xyz(ITEM_OFFSET, 0., -ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn(SceneBundle {
				scene: asset_server.load("kaykit/coin.glb#Scene0"),
				transform: Transform::from_xyz(0., 0.2, 0.).with_scale(scale),
				..default()
			});
		});
	commands
		.spawn((
			Name::new("Sword"),
			Sentence::new("silver sword"),
			Item,
			SpatialBundle {
				transform: Transform::from_xyz(-ITEM_OFFSET, 0., ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn(SceneBundle {
				scene: asset_server.load("kaykit/sword.glb#Scene0"),
				transform: Transform::from_xyz(0., 0.15, 0.).with_scale(scale),
				..default()
			});
		});
	commands
		.spawn((
			Name::new("Cheese"),
			Sentence::new("tasty cheese"),
			Item,
			SpatialBundle {
				transform: Transform::from_xyz(-ITEM_OFFSET, 0., -ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn(SceneBundle {
				scene: asset_server.load("kaykit/cheese.glb#Scene0"),
				transform: Transform::from_xyz(0., 0., 0.).with_scale(scale),
				..default()
			});
		});
}

fn rotate_items(time: Res<Time>, mut query: Query<&mut Transform, With<Item>>) {
	for mut transform in query.iter_mut() {
		transform.rotate(Quat::from_rotation_y(time.delta_seconds() * 0.5));
	}
}

fn set_player_sentence(
	mut commands: Commands,
	mut npc_events: EventWriter<OnNpcMessage>,
	mut events: EventReader<OnUserMessage>,
	query: Query<Entity, With<Player>>,
) {
	for ev in events.read() {
		commands
			.entity(query.iter().next().unwrap())
			.insert(Sentence::new(ev.0.clone()));

		npc_events.send(OnNpcMessage("ok".to_string()));
	}
}
