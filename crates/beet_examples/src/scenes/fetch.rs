use super::*;
use beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

pub fn fetch_npc(mut commands: Commands) {
	let Foxie {
		graph,
		idle_index,
		walk_index,
		..
	} = load_foxie();

	commands
		.spawn((
			Player,
			Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(0.01)),
			BundlePlaceholder::Scene("Fox.glb#Scene0".into()),
			graph,
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

			parent
				.spawn((
					Name::new("Fetch Behavior"),
					InsertOnTrigger::<AppReady, Running>::default(),
					SequenceSelector,
					Repeat,
				))
				.with_children(|parent| {
					parent
						.spawn((
							Name::new("Idle Or Fetch"),
							TriggerOnRun(AppReady),
							TargetAgent(agent),
							ScoreSelector::default(),
							// ScoreSelector::consuming(),
							AssetPlaceholder::<Bert>::new("default-bert.ron"),
							FindSentenceSteerTarget::<Collectable>::default(),
						))
						.with_children(|parent| {
							parent.spawn((
								Name::new("Idle"),
								Score::neutral(),
								TargetAgent(agent),
								SetAgentOnRun(Velocity::default()),
								PlayAnimation::new(idle_index).repeat_forever(),
								RunTimer::default(),
								InsertInDuration::new(
									RunResult::Success,
									Duration::from_secs(1),
								),
							));
							parent
								.spawn((
									Name::new("Fetch"),
									Score::default(),
									TargetAgent(agent),
									ScoreSteerTarget::new(10.),
									PlayAnimation::new(walk_index)
										.repeat_forever(),
									SequenceSelector,
									RemoveAgentOnRun::<Sentence>::default(),
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



pub fn fetch_scene(mut commands: Commands) {
	const ITEM_OFFSET: f32 = 2.;

	// camera
	commands.spawn((
		CameraDistance {
			width: ITEM_OFFSET * 1.4,
			offset: Vec3::new(0., 1.6, ITEM_OFFSET),
		},
		BundlePlaceholder::Camera3d
	));

	// items
	let scale = Vec3::splat(0.5);
	commands
		.spawn((
			Name::new("Potion"),
			Sentence::new("red healing potion"),
			Collectable,
			SpatialBundle {
				transform: Transform::from_xyz(ITEM_OFFSET, 0., ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0., 0.).with_scale(scale),
				BundlePlaceholder::Scene("kaykit/potion.glb#Scene0".into()),
			));
		});
	commands
		.spawn((
			Name::new("Coin"),
			Sentence::new("gold coin"),
			Collectable,
			SpatialBundle {
				transform: Transform::from_xyz(ITEM_OFFSET, 0., -ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0.2, 0.).with_scale(scale),
				BundlePlaceholder::Scene("kaykit/coin.glb#Scene0".into()),
			));
		});
	commands
		.spawn((
			Name::new("Sword"),
			Sentence::new("silver sword"),
			Collectable,
			SpatialBundle {
				transform: Transform::from_xyz(-ITEM_OFFSET, 0., ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0.15, 0.).with_scale(scale),
				BundlePlaceholder::Scene("kaykit/sword.glb#Scene0".into()),
			));
		});
	commands
		.spawn((
			Name::new("Cheese"),
			Sentence::new("tasty cheese"),
			Collectable,
			SpatialBundle {
				transform: Transform::from_xyz(-ITEM_OFFSET, 0., -ITEM_OFFSET),
				..default()
			},
		))
		.with_children(|parent| {
			parent.spawn((
				Transform::from_xyz(0., 0., 0.).with_scale(scale),
				BundlePlaceholder::Scene("kaykit/cheese.glb#Scene0".into()),
			));
		});
}
