use std::time::Duration;

use beetmash::prelude::*;
use bevy::{animation::RepeatAnimation, prelude::*};
use crate::beet::prelude::*;

pub struct Barbarian {
	pub graph: AnimationGraphPlaceholder,
	pub idle_clip: AssetPlaceholder<AnimationClip>,
	pub idle_index: AnimationNodeIndex,
	pub walk_clip: AssetPlaceholder<AnimationClip>,
	pub walk_index: AnimationNodeIndex,
}

impl Default for Barbarian {
	fn default() -> Self {
		let mut graph = AnimationGraphPlaceholder::default();

		let idle_clip = AssetPlaceholder::<AnimationClip>::new(
			"kaykit-adventurers/Barbarian.glb#Animation36",
		);
		let idle_index = graph.add_clip(idle_clip.clone(), 1.0, graph.root);
		let walk_clip = AssetPlaceholder::<AnimationClip>::new(
			"kaykit-adventurers/Barbarian.glb#Animation72",
		);
		let walk_index = graph.add_clip(walk_clip.clone(), 1.0, graph.root);

		Self {
			graph,
			idle_clip,
			idle_index,
			walk_clip,
			walk_index,
		}
	}
}


pub fn spawn_barbarian(mut commands: Commands) {
	let Barbarian {
		graph,
		idle_clip,
		idle_index,
		walk_clip,
		walk_index,
	} = default();

	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
			Name::new("Foxie"),
			Transform::from_scale(Vec3::splat(0.1)),
			BundlePlaceholder::Scene("kaykit-adventurers/Barbarian.glb#Scene0".into()),
			graph,
			AnimationTransitions::new(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Animation Behavior"),
					RunOnSpawn,
					SequenceFlow,
					Repeat::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						ContinueRun::default(),
						TargetAgent(agent),
						PlayAnimation::new(idle_index)
							.repeat(RepeatAnimation::Count(1))
							.with_transition_duration(transition_duration),
						idle_clip,
						TriggerOnAnimationEnd::new(
							idle_index,
							OnRunResult::success(),
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Walking"),
						ContinueRun::default(),
						TargetAgent(agent),
						PlayAnimation::new(walk_index)
							.repeat(RepeatAnimation::Count(4))
							.with_transition_duration(transition_duration),
						walk_clip,
						TriggerOnAnimationEnd::new(
							walk_index,
							OnRunResult::success(),
						)
						.with_transition_duration(transition_duration),
					));
				});
		});
}









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
