use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers action control-flow types for scene serialization.
///
/// Registers the unit-input instantiations of control-flow components
/// so that scenes containing action trees can be serialized and deserialized.
#[derive(Default)]
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
	fn build(&self, app: &mut App) {
		app
			// hierarchy types needed for scene serialization
			.register_type::<ChildOf>()
			.register_type::<Children>()
			// control-flow types
			.register_type::<ChildError>()
			.register_type::<CallOnSpawn<(), Outcome>>()
			.add_systems(Update, call_on_spawn::<(), Outcome>)
			.register_type::<ExcludeErrors>()
			.register_type::<Sequence<(), ()>>()
			.register_type::<Infallible>()
			.register_type::<Parallel<(), ()>>()
			.register_type::<HighestScore<(), ()>>()
			.register_type::<Score>()
			// agent resolution types
			.register_type::<ActionOf>()
			.register_type::<Actions>()
			.register_type::<TargetEntity>()
			// leaf / util actions
			.register_type::<EndWith<Outcome>>()
			.register_type::<Log>()
			.register_type::<SucceedTimes>()
			.register_type::<EndInDuration<Outcome>>()
			.register_type::<RunNext>();
	}
}
