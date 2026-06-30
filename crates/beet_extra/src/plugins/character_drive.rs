//! The wgpu render body for the agnostic `SetDrive` action.
//!
//! `<SetDrive>` (in `beet_action`) writes a body's commanded [`DifferentialDrive`] via
//! `AgentQuery`; this module is the render-side interpretation — a [`CharacterDrive`]
//! that integrates that command into its own `Transform` and crossfades a walk/idle
//! animation. The Alvik robot interprets the identical command by mapping it to its
//! wheels, so the *same* `<SetDrive>` square patrol drives the on-screen fox and the
//! real robot.
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

/// A wgpu body driven by a [`DifferentialDrive`] (the commanded motion a `<SetDrive>`
/// leaf writes): each frame it turns by the angular rate and steps forward by the
/// linear rate, integrating its own `Transform`, and crossfades its walk/idle
/// animation as the command changes. The render-side counterpart to the robot's wheel
/// mapper — the same command, a different body.
///
/// The command's `linear` is mm/s and the render world treats one mm as one world
/// unit, so the same `<SetDrive linear=60 angular=90>` square patrol that crawls a 6 cm
/// square on the robot walks a 60-unit square here. `#[require(DifferentialDrive)]`
/// declares the command on the body at spawn, satisfying `<SetDrive>`'s requirement.
/// Carries the two clip paths it switches between (defaulting to the fox's idle/walk)
/// so a `.bsx` authors a bare `{CharacterDrive}` and the animation "just works".
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DifferentialDrive)]
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
	mut query: Query<(&DifferentialDrive, &mut Transform), With<CharacterDrive>>,
) {
	let dt = time.delta_secs();
	for (command, mut transform) in query.iter_mut() {
		let angular = command.angular.as_rad_per_sec();
		if angular != 0.0 {
			transform.rotate_y(angular * dt);
		}
		let linear = command.linear.as_mm_per_sec();
		if linear != 0.0 {
			let forward = transform.rotation * Vec3::Z;
			transform.translation += forward * linear * dt;
		}
	}
}

/// Crossfades the body's animation whenever its commanded [`DifferentialDrive`]
/// changes: walk while moving *or* turning, idle only when fully stopped. Keyed on
/// `Changed<DifferentialDrive>` — which only happens when a new `Drive` step runs — so
/// the gait tracks the command, transitioning exactly at the behaviour-tree step
/// boundaries with no per-frame thrash. Resolves the clip against the agent's
/// `AnimationGraphClips` and plays it on the `AnimationPlayer` the glb spawns as a
/// descendant, the same resolution `PlayAnimation` uses.
pub(crate) fn drive_character_animation(
	query: Query<
		(Entity, &DifferentialDrive, &CharacterDrive),
		Changed<DifferentialDrive>,
	>,
	graph_clips: Query<&AnimationGraphClips>,
	children: Query<&Children>,
	mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
	for (entity, command, character) in query.iter() {
		let moving = command.linear.as_mm_per_sec().abs() > f32::EPSILON
			|| command.angular.as_deg_per_sec().abs() > f32::EPSILON;
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
/// follow the agent's `<SetDrive>` velocity.
pub(crate) fn character_drive_plugin(app: &mut App) {
	app.register_type::<CharacterDrive>()
		.add_systems(Update, (drive_character, drive_character_animation));
}
