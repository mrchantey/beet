//! The environment-agnostic drive example: one behaviour tree, the same `Drive`
//! leaf, run unchanged in every environment, with each environment interpreting the
//! command for the body it owns.
//!
//! ```bsx
//! <Sequence {RunOnLoad}>
//!   <Drive linear=60. angular=0./>  <EndInDuration duration="1s"/>
//!   <Drive linear=0.  angular=90./> <EndInDuration duration="1s"/>
//!   <Drive linear=0.  angular=0./>
//! </Sequence>
//! ```
//!
//! `<Drive>` writes its `(linear, angular)` onto the agent's persistent
//! [`DriveCommand`] and logs the step, then passes instantly; an [`EndInDuration`]
//! sibling provides the dwell, so a `<Sequence>` reads as "drive like this for N
//! seconds, then the next thing". The command persists between steps, exactly like
//! the robot's `DifferentialDrive`.
//!
//! Each environment reads that one command differently:
//! - headless ([`just beet --main=demo/08-behavior.bsx`]): the `Drive` leaf's log is
//!   the output — the behaviour prints its steps with no body at all.
//! - wgpu ([`CharacterDrive`]): a per-frame system integrates the command into the
//!   fox's `Transform` and crossfades its walk/idle animation as the command changes.
//! - the Alvik robot (`../beet_esp`): the firmware's own `Drive`/`ApplyDrive` maps the
//!   identical markup to wheel velocities.
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Persistent drive command on a driven body: a forward speed and a turn rate.
///
/// Written by the [`Drive`] leaf onto its agent and read each frame by whatever owns
/// the body — here the wgpu [`CharacterDrive`]; on the Alvik robot, the firmware's own
/// motor mapper. The render and robot analogue of one shared idea: "what motion is the
/// body currently commanded to make".
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DriveCommand {
	/// Forward speed, units/s (negative = reverse).
	pub linear: f32,
	/// Turn rate, deg/s (positive = left).
	pub angular: f32,
}

/// `<Drive linear=.. angular=../>` — the environment-agnostic motor leaf. Logs the
/// step (so a headless run prints the behaviour) and applies its `(linear, angular)`
/// to the agent's persistent [`DriveCommand`], then passes instantly. Pair with an
/// [`EndInDuration`] in a `<Sequence>` to "drive like this for N seconds".
///
/// The config is a component field, read by [`DriveAction`]; it carries no body of its
/// own, so the same leaf drives a logged sequence, a rendered fox, or a real robot.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DriveAction)]
pub struct Drive {
	/// Forward speed, units/s (negative = reverse).
	pub linear: f32,
	/// Turn rate, deg/s (positive = left).
	pub angular: f32,
}

/// Reads the caller's [`Drive`], logs the step, applies it to the agent's
/// [`DriveCommand`] (if the agent has one), then passes. Resolving the command onto
/// the agent rather than the leaf is what lets the motion persist across the
/// `EndInDuration` dwell that follows.
#[action(default)]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn DriveAction(cx: ActionContext) -> Result<Outcome> {
	let drive = cx.caller.get_cloned::<Drive>().await?;
	info!("Drive: linear={} angular={}", drive.linear, drive.angular);
	let world = cx.world();
	// the agent is the body the leaf drives: an `ActionOf` body (the fox) or the
	// behaviour root. A headless tree has no `DriveCommand` to write, so this is a
	// no-op there and the log above is the whole story.
	let agent = AgentQuery::entity_async(&world, cx.caller.id()).await;
	world
		.entity(agent)
		.with(move |mut agent| {
			if let Some(mut command) = agent.get_mut::<DriveCommand>() {
				command.linear = drive.linear;
				command.angular = drive.angular;
			}
		})
		.await?;
	Outcome::PASS.xok()
}

// ── wgpu body ────────────────────────────────────────────────
//
// The render-side interpretation of `DriveCommand`: a character that integrates the
// command into its own `Transform` and animates itself. Only compiled with the render
// stack, so the headless `Drive` above stays dependency-free.
#[cfg(feature = "bevy_default")]
mod character {
	use super::DriveCommand;
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

	/// Tag for a body driven by [`DriveCommand`] in the wgpu environment: each frame
	/// it turns by the commanded angular rate and steps forward by the linear rate,
	/// integrating its own `Transform`, and crossfades its walk/idle animation as the
	/// command changes. The render-side counterpart to the robot's motor mapper — the
	/// same `DriveCommand`, a different body.
	///
	/// Carries the two clip paths it switches between (defaulting to the fox's
	/// `Animation0`/`Animation1`) so a `.bsx` authors a bare `{CharacterDrive}` and the
	/// animations "just work".
	#[derive(Debug, Clone, Component, Reflect)]
	#[reflect(Component, Default)]
	#[require(DriveCommand)]
	pub struct CharacterDrive {
		/// Clip played while stopped (`linear` ≈ 0).
		pub idle: String,
		/// Clip played while moving.
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

	/// Integrates each [`CharacterDrive`]'s [`DriveCommand`] into its `Transform`:
	/// rotate about `+Y` by the angular rate, then step along the body's visual forward
	/// (GLTF `+Z`, matching `RotateToVelocity3d`) by the linear rate. A turn-in-place
	/// step (`linear = 0`) only rotates; a straight step only translates; the square
	/// patrol is the two alternating.
	pub(crate) fn drive_character(
		time: When<Res<Time>>,
		mut query: Query<(&DriveCommand, &mut Transform), With<CharacterDrive>>,
	) {
		let dt = time.delta_secs();
		for (command, mut transform) in query.iter_mut() {
			if command.angular != 0.0 {
				transform.rotate_y(command.angular.to_radians() * dt);
			}
			if command.linear != 0.0 {
				let forward = transform.rotation * Vec3::Z;
				transform.translation += forward * command.linear * dt;
			}
		}
	}

	/// Crossfades the character's animation whenever its [`DriveCommand`] changes:
	/// walk while moving, idle while stopped. Keyed on `Changed<DriveCommand>` — which
	/// only fires when a new `Drive` step runs — so the gait tracks the commanded
	/// velocity, transitioning exactly at the behaviour-tree step boundaries with no
	/// per-frame thrash. Resolves the clip against the agent's `AnimationGraphClips`
	/// and plays it on the `AnimationPlayer` the glb spawns as a descendant, the same
	/// resolution `PlayAnimation` uses.
	pub(crate) fn drive_character_animation(
		query: Query<
			(Entity, &DriveCommand, &CharacterDrive),
			Changed<DriveCommand>,
		>,
		graph_clips: Query<&AnimationGraphClips>,
		children: Query<&Children>,
		mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
	) {
		for (entity, command, character) in query.iter() {
			let clip = if command.linear.abs() > f32::EPSILON {
				&character.walk
			} else {
				&character.idle
			};
			let Ok(clips) = graph_clips.get(entity) else {
				continue;
			};
			let Some(index) = clips.index(clip.as_str()) else {
				continue;
			};
			// the player is a descendant of the glb scene root, spawned async; once it
			// exists, crossfade to the gait clip and loop it.
			for descendant in children.iter_descendants_inclusive(entity) {
				if let Ok((mut player, mut transitions)) =
					players.get_mut(descendant)
				{
					transitions
						.play(&mut player, index, TRANSITION)
						.set_repeat(RepeatAnimation::Forever);
					break;
				}
			}
		}
	}
}

#[cfg(feature = "bevy_default")]
pub use character::CharacterDrive;

/// Registers the agnostic drive types, so a `.bsx` declaring `<Drive>` resolves: the
/// headless `Drive`/`DriveCommand` always, and — with the render stack — the wgpu
/// [`CharacterDrive`] body plus its per-frame movement and animation systems.
pub struct DriveExamplesPlugin;

impl Plugin for DriveExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Drive>().register_type::<DriveCommand>();
		#[cfg(feature = "bevy_default")]
		app.register_type::<character::CharacterDrive>().add_systems(
			Update,
			(
				character::drive_character,
				character::drive_character_animation,
			),
		);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn registers_drive_tag() {
		let mut app = App::new();
		app.add_plugins(DriveExamplesPlugin);
		let registry = app.world().resource::<AppTypeRegistry>().read();
		registry
			.get_with_short_type_path("Drive")
			.is_some()
			.xpect_true();
		registry
			.get_with_short_type_path("DriveCommand")
			.is_some()
			.xpect_true();
	}
}
