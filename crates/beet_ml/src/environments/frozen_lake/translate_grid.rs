use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::f32::consts::TAU;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
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
		(Entity, &TranslateGrid, &TargetAgent, &RunTimer),
		With<Running>,
	>,
) {
	for (entity, action, agent, run_timer) in query.iter() {
		let Ok((mut transform, pos, dir, grid_to_world)) =
			agents.get_mut(**agent)
		else {
			continue;
		};
		let from = grid_to_world.world_pos(**pos);
		let to = pos.saturating_add_signed((*dir).into());
		let to = grid_to_world.world_pos(to);

		let t = run_timer.last_started.elapsed().as_secs_f32()
			/ action.anim_duration.as_secs_f32();

		let dir_vec: Vec3 = (*dir).into();
		transform.look_to(-dir_vec, Vec3::Y);

		if t < 1.0 {
			let mut pos = from.lerp(to, t);
			pos.y = grid_to_world.offset.y
				+ bounce(t, 1) * grid_to_world.cell_width * 0.25;
			transform.translation = pos;
		} else {
			transform.translation = to;
			commands.entity(entity).insert(RunResult::Success);
		}
	}
}

fn bounce(t: f32, n: i32) -> f32 {
	let t = t * (n as f32) * TAU;
	let bounce = t.sin().abs();
	bounce
}

impl ActionMeta for TranslateGrid {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl ActionSystems for TranslateGrid {
	fn systems() -> SystemConfigs { translate_grid.in_set(TickSet) }
}
