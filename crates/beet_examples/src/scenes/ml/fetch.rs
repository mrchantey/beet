use crate::beet::prelude::*;
use crate::prelude::*;
use beetmash::core::scenes::Foxie;
use beetmash::prelude::*;
use bevy::prelude::*;


pub fn fetch_npc(mut commands: Commands) {
	let Foxie {
		graph,
		idle_index,
		walk_index,
		..
	} = default();

	commands
		.spawn((
			Name::new("Fox"),
			Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(0.01)),
			BundlePlaceholder::Scene("misc/fox.glb#Scene0".into()),
			graph,
			AnimationTransitions::new(),
			RotateToVelocity3d::default(),
			ForceBundle::default(),
			SteerBundle {
				max_force: MaxForce(0.05),
				max_speed: MaxSpeed(2.),
				..default()
			},
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Fetch Behavior"),
					TargetAgent(agent),
					AssetRunOnReady::<Bert>::new("ml/default-bert.ron"),
					InsertSentenceOnUserInput::default(),
					InsertSentenceSteerTarget::<Collectable>::default(),
					RunOnSteerTargetInsert::new_with_source(agent),
					RunOnSteerTargetRemove::new_with_source(agent),
					ScoreFlow::default(),
					RemoveOnRunResult::<Sentence>::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						ScoreProvider::NEUTRAL,
						TargetAgent(agent),
						PlayAnimation::new(idle_index).repeat_forever(),
					));
					parent.spawn((
						Name::new("Fetch"),
						TargetAgent(agent),
						SteerTargetScoreProvider {
							min_radius: 1.,
							max_radius: 10.,
						},
						PlayAnimation::new(walk_index).repeat_forever(),
						InsertOnRun::<Velocity>::new_with_target(agent),
						Seek::default(),
						EndOnArrive::new(1.),
						RemoveOnRunResult::<SteerTarget>::new_with_target(
							agent,
						),
						RemoveOnRunResult::<Velocity>::new_with_target(agent),
					));
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
		BundlePlaceholder::Camera3d,
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
				BundlePlaceholder::Scene("kaykit/potion.glb#Scene0".into()),
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
				BundlePlaceholder::Scene("kaykit/coin.glb#Scene0".into()),
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
				BundlePlaceholder::Scene("kaykit/sword.glb#Scene0".into()),
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
				BundlePlaceholder::Scene("kaykit/cheese.glb#Scene0".into()),
			));
		});
}
