use beet::prelude::*;
use crate::prelude::*;
use beetmash::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

pub fn spawn_barbarian(mut commands: Commands) {
	let mut graph = AnimationGraphPlaceholder::default();

	let idle_animation_bundle = AnimationActionBundle::new(
		&mut graph,
		"kaykit-adventurers/Barbarian.glb#Animation36",
	)
	.repeat(RepeatAnimation::Forever);

	let cheer_animation_bundle = AnimationActionBundle::new(
		&mut graph,
		"kaykit-adventurers/Barbarian.glb#Animation22",
	);

	let mut idle_behavior = Entity::PLACEHOLDER;

	commands
		.spawn((
			Name::new("Barbarian"),
			// Transform::from_scale(Vec3::splat(0.1)),
			BundlePlaceholder::Scene(
				"kaykit-adventurers/Barbarian_NoProps.glb#Scene0".into(),
			),
			AssetLoadBlockAppReady,
			graph,
			AnimationTransitions::default(),
			RenderLayers::layer(RENDER_TEXTURE_LAYER),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			let emote_bubble = spawn_emote_bubble(&mut parent.spawn((
				Transform::from_xyz(0.5, 2.5, 0.5),
				Visibility::Hidden,
				RenderLayers::layer(RENDER_TEXTURE_LAYER),
			)));

			idle_behavior = parent
				.spawn((
					Name::new("Idle"),
					TargetAgent(agent),
					idle_animation_bundle,
				))
				.id();

			parent.spawn((
				Name::new("Respond To User"),
				EndOnRun::success().with_target(idle_behavior),
				InsertSentenceOnUserInput::default(),
				RunOnInsertSentence::default(),
				InsertOnRun::new(Visibility::Visible).with_target(emote_bubble),
				InsertOnRunResult::new(Visibility::Hidden)
					.with_target(emote_bubble),
				TargetAgent(agent),
				cheer_animation_bundle,
				RunOnRunResult::new_with_target(idle_behavior),
			));
		})
		.insert(RunOnSceneReady::new_with_target(idle_behavior));
}





// pub fn disable_barbarian(
// 	mut commands: Commands,
// 	mut events: EventReader<AssetEvent<Scene>>,
// 	query: Populated<(Entity, &SceneRoot)>,
// 	names: Query<&Name>,
// 	children: Query<&Children>,
// ) {
// 	for ev in events.read() {
// 		let AssetEvent::LoadedWithDependencies { id } = ev else {
// 			continue;
// 		};
// 		let Some((entity, _)) =
// 			query.iter().find(|(_, scene)| scene.id() == *id)
// 		else {
// 			continue;
// 		};

// 		let to_disable: HashSet<&'static str> =
// 			vec!["Mug"].into_iter().collect();

// 		for entity in children.iter_descendants(entity) {
// 			if let Ok(name) = names.get(entity) {
// 				println!("it has name: {}", name);
// 				if to_disable.contains(name.as_str()) {
// 					commands.entity(entity).insert(Visibility::Hidden);
// 				}
// 			}else{
// 				println!("it has no name");
// 			}
// 		}
// 	}
// }






/*

# meshes
1H_Axe
2H_Axe
Mug


https://kaylousberg.itch.io/kaykit-adventurers
  Idle: 36,
  Walking_A: 72,
  Walking_B: 73,
  Walking_C: 75,
  Walking_Backwards: 74,
  Running_A: 48,
  Running_B: 49,
  Running_Strafe_Right: 51,
  Running_Strafe_Left: 50,
  Jump_Full_Short: 39,
  Jump_Full_Long: 38,
  Jump_Start: 42,
  Jump_Idle: 40,
  Jump_Land: 41,
  Dodge_Right: 30,
  Dodge_Left: 29,
  Dodge_Forward: 28,
  Dodge_Backward: 27,
  PickUp: 47,
  Use_Item: 71,
  Throw: 65,
  Interact: 37,
  Cheer: 22,
  Hit_A: 34,
  Hit_B: 35,
  Death_A: 23,
  Death_A_Pose: 24,
  Death_B: 25,
  Death_B_Pose: 26,
  1H_Melee_Attack_Chop: 0,
  1H_Melee_Attack_Slice_Diagonal: 1,
  1H_Melee_Attack_Slice_Horizontal: 2,
  1H_Melee_Attack_Stab: 3,
  2H_Melee_Idle: 13,
  2H_Melee_Attack_Chop: 8,
  2H_Melee_Attack_Slice: 9,
  2H_Melee_Attack_Stab: 12,
  2H_Melee_Attack_Spin: 10,
  2H_Melee_Attack_Spinning: 11,
  Dualwield_Melee_Attack_Chop: 31,
  Dualwield_Melee_Attack_Slice: 32,
  Dualwield_Melee_Attack_Stab: 33,
  Unarmed_Idle: 66,
  Unarmed_Pose: 70,
  Unarmed_Melee_Attack_Punch_A: 68,
  Unarmed_Melee_Attack_Punch_B: 69,
  Unarmed_Melee_Attack_Kick: 67,
  Block: 18,
  Blocking: 21,
  Block_Hit: 20,
  Block_Attack: 19,
  1H_Ranged_Aiming: 4,
  1H_Ranged_Shoot: 6,
  1H_Ranged_Shooting: 7,
  1H_Ranged_Reload: 5,
  2H_Ranged_Aiming: 14,
  2H_Ranged_Shoot: 16,
  2H_Ranged_Shooting: 17,
  2H_Ranged_Reload: 15,
  Spellcast_Shoot: 62,
  Spellcast_Raise: 61,
  Spellcast_Long: 60,
  Spellcast_Charge: -1,
  Lie_Down: 43,
  Lie_Idle: 44,
  Lie_Pose: 45,
  Lie_StandUp: 46,
  Sit_Chair_Down: 52,
  Sit_Chair_Idle: 53,
  Sit_Chair_Pose: 54,
  Sit_Chair_StandUp: 55,
  Sit_Floor_Down: 56,
  Sit_Floor_Idle: 57,
  Sit_Floor_Pose: 58,
  Sit_Floor_StandUp: 59
*/
