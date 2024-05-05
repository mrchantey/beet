use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;

impl ActionMeta for Translate {
	fn graph_role(&self) -> GraphRole { GraphRole::Agent }
}

impl ActionSystems for Translate {
	fn systems() -> SystemConfigs { translate.in_set(TickSet) }
}

#[derive(
	Debug, Default, Clone, PartialEq, Component, Reflect, InspectorOptions,
)]
#[reflect(Default, Component, ActionMeta, InspectorOptions)]
/// Applies constant translation, multiplied by [`Time::delta_seconds`]
pub struct Translate {
	/// Translation to apply, in meters per second
	#[inspector(min=-2., max=2., step=0.1)]
	pub translation: Vec3,
}

impl Translate {
	pub fn new(translation: Vec3) -> Self { Self { translation } }
}

fn translate(
	mut _commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Query<(&TargetAgent, &Translate), With<Running>>,
) {
	for (target, translate) in query.iter() {
		if let Some(mut transform) =
			transforms.get_mut(**target).ok_or(|e| log::warn!("{e}"))
		{
			transform.translation +=
				translate.translation * time.delta_seconds();
		}
	}
}
