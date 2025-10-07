use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::f32::consts::TAU;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct TranslateGrid {
	pub anim_duration: Duration,
}

impl TranslateGrid {
	pub fn new(anim_duration: Duration) -> Self { Self { anim_duration } }
}

impl Default for TranslateGrid {
	fn default() -> Self {
		Self {
			anim_duration: Duration::from_secs(1),
		}
	}
}


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
	for (action, translate_grid, run_timer) in query.iter() {
		let (mut transform, mut grid_pos, dir, grid_to_world) =
			agents.get_mut(action)?;
		let from_world = grid_to_world.world_pos(**grid_pos);
		let to_grid = grid_to_world.clamped_add(**grid_pos, (*dir).into());
		let to_world = grid_to_world.world_pos(to_grid);

		let t = run_timer.last_run.elapsed().as_secs_f32()
			/ translate_grid.anim_duration.as_secs_f32();

		let dir_vec: Vec3 = (*dir).into();

		let to_rot = transform.looking_to(-dir_vec, Vec3::Y).rotation;
		// let t_rot = t.min(0.1) * 10.;
		let t_rot = t;
		transform.rotation = transform.rotation.slerp(to_rot, t_rot);

		if t < 1.0 {
			let mut pos = from_world.lerp(to_world, t);
			pos.y = grid_to_world.offset.y
				+ bounce(t, 1) * grid_to_world.cell_width * 0.25;
			transform.translation = pos;
		} else {
			transform.translation = to_world;
			**grid_pos = to_grid;
			commands.entity(action).trigger_payload(Outcome::Pass);
		}
	}
	Ok(())
}

fn bounce(t: f32, n: i32) -> f32 {
	let t = t * (n as f32) * TAU;
	let bounce = t.sin().abs();
	bounce
}
