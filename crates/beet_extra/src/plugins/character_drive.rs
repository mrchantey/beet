//! The wgpu render body for the agnostic `Drive` action.
//!
//! `<Drive>` (in `beet_action`) writes a body's commanded [`LinearVelocity`] +
//! [`AngularVelocity`] via `AgentQuery`; this module is the render-side
//! interpretation — a [`CharacterDrive`] that integrates that velocity into its own
//! `Transform` and crossfades a walk/idle animation. The Alvik robot interprets the
//! identical velocity by mapping it to its wheels, so the *same* `<Drive>` square
//! patrol drives the on-screen fox and the real robot.
use crate::beet::prelude::*;
use beet_core::prelude::When;
use beet_core::prelude::*;
// `AnimationPlayer`/`AnimationTransitions` ride in on the bevy prelude glob above;
// `RepeatAnimation` does not, so it is named explicitly (as `PlayAnimation` does).
use bevy::animation::RepeatAnimation;
use core::time::Duration;

/// The crossfade between the walk and idle clips, matching `beet_spatial`'s
/// `PlayAnimation` default so the gait switch blends rather than snaps.
const TRANSITION: Duration = Duration::from_millis(250);

/// A wgpu body driven by [`LinearVelocity`] + [`AngularVelocity`] (the commanded
/// motion a `<Drive>` leaf writes): each frame it turns by the angular rate and
/// steps forward by the linear rate, integrating its own `Transform`, and crossfades
/// its walk/idle animation as the velocity changes. The render-side counterpart to
/// the robot's wheel mapper — the same velocity, a different body.
///
/// `LinearVelocity` is mm/s and the render world treats one mm as one world unit, so
/// the same `<Drive linear=60 angular=90>` square patrol that crawls a 6 cm square on
/// the robot walks a 60-unit square here. Carries the two clip paths it switches
/// between (defaulting to the fox's idle/walk) so a `.bsx` authors a bare
/// `{CharacterDrive}` and the animation "just works".
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(LinearVelocity, AngularVelocity)]
pub struct CharacterDrive {
	/// Clip played while stopped (no linear *or* angular velocity).
	pub idle: String,
	/// Clip played while moving or turning.
	pub walk: String,
}

impl Default for CharacterDrive {
	fn default() -> Self {
		Self {
			idle: "misc/fox.glb#Animation0".into(),
			walk: "misc/fox.glb#Animation1".into(),
		}
	}
}

/// Integrates each [`CharacterDrive`]'s commanded velocity into its `Transform`:
/// rotate about `+Y` by the angular rate, then step along the body's visual forward
/// (GLTF `+Z`, matching `RotateToVelocity3d`) by the linear rate. A turn-in-place
/// step (`linear = 0`) only rotates; a straight step only translates; the square
/// patrol is the two alternating.
pub(crate) fn drive_character(
	time: When<Res<Time>>,
	mut query: Query<
		(&LinearVelocity, &AngularVelocity, &mut Transform),
		With<CharacterDrive>,
	>,
) {
	let dt = time.delta_secs();
	for (linear, angular, mut transform) in query.iter_mut() {
		let angular = angular.as_rad_per_sec();
		if angular != 0.0 {
			transform.rotate_y(angular * dt);
		}
		let linear = linear.as_mm_per_sec();
		if linear != 0.0 {
			let forward = transform.rotation * Vec3::Z;
			transform.translation += forward * linear * dt;
		}
	}
}

/// Crossfades the body's animation whenever its commanded velocity changes: walk
/// while moving *or* turning, idle only when fully stopped. Keyed on a change to
/// either velocity — which only happens when a new `Drive` step runs — so the gait
/// tracks the command, transitioning exactly at the behaviour-tree step boundaries
/// with no per-frame thrash. Resolves the clip against the agent's
/// `AnimationGraphClips` and plays it on the `AnimationPlayer` the glb spawns as a
/// descendant, the same resolution `PlayAnimation` uses.
pub(crate) fn drive_character_animation(
	query: Query<
		(Entity, &LinearVelocity, &AngularVelocity, &CharacterDrive),
		Or<(Changed<LinearVelocity>, Changed<AngularVelocity>)>,
	>,
	graph_clips: Query<&AnimationGraphClips>,
	children: Query<&Children>,
	mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
	for (entity, linear, angular, character) in query.iter() {
		let moving = linear.as_mm_per_sec().abs() > f32::EPSILON
			|| angular.as_deg_per_sec().abs() > f32::EPSILON;
		let clip = if moving { &character.walk } else { &character.idle };
		let Ok(clips) = graph_clips.get(entity) else {
			continue;
		};
		let Some(index) = clips.index(clip.as_str()) else {
			continue;
		};
		// the player is a descendant of the glb scene root, spawned async; once it
		// exists, crossfade to the gait clip and loop it.
		for descendant in children.iter_descendants_inclusive(entity) {
			if let Ok((mut player, mut transitions)) = players.get_mut(descendant)
			{
				transitions
					.play(&mut player, index, TRANSITION)
					.set_repeat(RepeatAnimation::Forever);
				break;
			}
		}
	}
}

/// Registers the render `CharacterDrive` body and its per-frame movement + animation
/// systems, so a scene `.bsx` can spread `{CharacterDrive}` onto a model to make it
/// follow the agent's `<Drive>` velocity.
pub(crate) fn character_drive_plugin(app: &mut App) {
	app.register_type::<CharacterDrive>()
		.add_systems(Update, (drive_character, drive_character_animation));
}
