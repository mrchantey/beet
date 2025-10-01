use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::input::common_conditions::input_toggle_active;
use beet_core::prelude::*;
use std::marker::PhantomData;


/// Provides debug visualization for the `Separate`, `Align`, and `Cohere` actions.
pub struct DebugGroupSteerPlugin<M> {
	toggle_key: KeyCode,
	phantom: PhantomData<M>,
}

impl Default for DebugGroupSteerPlugin<GroupSteerAgent> {
	fn default() -> Self {
		Self {
			toggle_key: KeyCode::KeyD,
			phantom: PhantomData,
		}
	}
}

impl<M: 'static + Send + Sync> Plugin for DebugGroupSteerPlugin<M> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			debug_group_steer::<M>
				.run_if(input_toggle_active(false, self.toggle_key)),
		);
	}
}

fn debug_group_steer<M: 'static + Send + Sync>(
	mut gizmos: Gizmos,
	transforms: Query<&Transform>,
	separate: Query<(&Running, &Separate<M>)>,
	align: Query<(&Running, &Align<M>)>,
	cohere: Query<(&Running, &Cohere<M>)>,
) {
	for (running, params) in separate.iter() {
		if let Ok(transform) = transforms.get(running.origin) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::AMBER_500,
			);
		}
	}

	for (running, params) in align.iter() {
		if let Ok(transform) = transforms.get(running.origin) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::GREEN_500,
			);
		}
	}

	for (running, params) in cohere.iter() {
		if let Ok(transform) = transforms.get(running.origin) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::CYAN_500,
			);
		}
	}
}
