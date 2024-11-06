use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Provides debug visualization for the `Separate`, `Align`, and `Cohere` actions.
pub struct DebugGroupSteerPlugin<M: GenericActionComponent> {
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

impl<M: GenericActionComponent> Plugin for DebugGroupSteerPlugin<M> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			debug_group_steer::<M>
				.run_if(input_toggle_active(false, self.toggle_key)),
		);
	}
}

fn debug_group_steer<M: GenericActionComponent>(
	mut gizmos: Gizmos,
	transforms: Query<&Transform>,
	separate: Query<(&TargetEntity, &Separate<M>)>,
	align: Query<(&TargetEntity, &Align<M>)>,
	cohere: Query<(&TargetEntity, &Cohere<M>)>,
) {
	for (agent, params) in separate.iter() {
		if let Ok(transform) = transforms.get(**agent) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::AMBER_500,
			);
		}
	}

	for (agent, params) in align.iter() {
		if let Ok(transform) = transforms.get(**agent) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::GREEN_500,
			);
		}
	}

	for (agent, params) in cohere.iter() {
		if let Ok(transform) = transforms.get(**agent) {
			gizmos.circle_2d(
				Isometry2d::from_translation(transform.translation.xy()),
				params.radius,
				tailwind::CYAN_500,
			);
		}
	}
}
