use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(translate_grid.in_set(TickSet))]
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


fn translate_grid(
	mut commands: Commands,
	mut agents: Query<(
		&mut Transform,
		&mut GridPos,
		&GridDirection,
		&GridToWorld,
	)>,
	query: Query<
		(Entity, &TranslateGrid, &TargetEntity, &RunTimer),
		With<Running>,
	>,
) {
	for (entity, action, agent, run_timer) in query.iter() {
		let Ok((mut transform, mut grid_pos, dir, grid_to_world)) =
			agents.get_mut(**agent)
		else {
			continue;
		};
		let from_world = grid_to_world.world_pos(**grid_pos);
		let to_grid = grid_to_world.clamped_add(**grid_pos, (*dir).into());
		let to_world = grid_to_world.world_pos(to_grid);

		let t = run_timer.last_started.elapsed().as_secs_f32()
			/ action.anim_duration.as_secs_f32();

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
			commands.entity(entity).trigger(OnRunResult::success());
		}
	}
}

fn bounce(t: f32, n: i32) -> f32 {
	let t = t * (n as f32) * TAU;
	let bounce = t.sin().abs();
	bounce
}
