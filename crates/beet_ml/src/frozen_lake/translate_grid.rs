use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::f32::consts::TAU;
use std::time::Duration;

/// Long-running action: animates the agent walking between grid cells.
///
/// Stays [`Running`] while the agent slides from its current cell to the
/// cell in the direction of its [`GridDirection`]. On completion, the
/// agent's [`GridPos`] is updated and the run ends with [`Outcome::PASS`].
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct TranslateGrid {
	/// Duration of the per-cell animation.
	pub anim_duration: Duration,
}

impl TranslateGrid {
	/// Create a [`TranslateGrid`] with the given animation duration.
	pub fn new(anim_duration: Duration) -> Self { Self { anim_duration } }
}

impl Default for TranslateGrid {
	fn default() -> Self {
		Self {
			anim_duration: Duration::from_secs(1),
		}
	}
}

/// Per-frame system: interpolates each [`Running`] [`TranslateGrid`]
/// agent toward the next cell, with a sine-wave bounce.
pub(crate) fn translate_grid(
	mut commands: Commands,
	mut agents: AgentQuery<(
		&mut Transform,
		&mut GridPos,
		&GridDirection,
		&GridToWorld,
	)>,
	query: Query<(Entity, &TranslateGrid, &RunTimer), With<Running>>,
) -> Result {
	for (action, translate, run_timer) in query.iter() {
		let (mut transform, mut grid_pos, dir, grid_to_world) =
			agents.get_mut(action)?;
		let from_world = grid_to_world.world_pos(**grid_pos);
		let to_grid = grid_to_world.clamped_add(**grid_pos, (*dir).into());
		let to_world = grid_to_world.world_pos(to_grid);

		let t = run_timer.last_run.elapsed().as_secs_f32()
			/ translate.anim_duration.as_secs_f32();

		// 1. rotate to face direction of travel
		let dir_vec: Vec3 = (*dir).into();
		let to_rot = transform.looking_to(-dir_vec, Vec3::Y).rotation;
		transform.rotation = transform.rotation.slerp(to_rot, t);

		// 2. translate, with a bounce on the y-axis until the lerp completes
		if t < 1.0 {
			let mut pos = from_world.lerp(to_world, t);
			pos.y = grid_to_world.offset.y
				+ bounce(t, 1) * grid_to_world.cell_width * 0.25;
			transform.translation = pos;
		} else {
			transform.translation = to_world;
			**grid_pos = to_grid;
			commands.entity(action).queue(EndRun(Outcome::PASS));
		}
	}
	Ok(())
}

// half-sine bounce, repeating `n` times across t ∈ [0,1]
fn bounce(t: f32, n: i32) -> f32 { (t * n as f32 * TAU).sin().abs() }
