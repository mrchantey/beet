use beet_flow::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

/// Applies constant translation, multiplied by [`Time::delta_secs`]
#[derive(Debug, Default, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(translate.in_set(TickSet))]
pub struct Translate {
	/// Translation to apply, in meters per second
	// #[inspector(min=-2., max=2., step=0.1)]
	pub translation: Vec3,
}

impl Translate {
	pub fn new(translation: Vec3) -> Self { Self { translation } }
}

fn translate(
	mut _commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Query<(&TargetEntity, &Translate), With<Running>>,
) {
	for (target, translate) in query.iter() {
		if let Some(mut transform) =
			transforms.get_mut(**target).ok_or(|e| sweet::elog!("{e}"))
		{
			transform.translation += translate.translation * time.delta_secs();
		}
	}
}
