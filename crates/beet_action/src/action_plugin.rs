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
		app.init_plugin::<MinimalTypesPlugin>()
			// control-flow types
			.register_type::<ChildError>()
			.register_type::<CallOnSpawn<(), Outcome>>()
			.add_systems(Update, call_on_spawn::<(), Outcome>)
			.register_type::<ExcludeErrors>()
			.register_type::<Sequence<(), ()>>()
			.register_type::<Repeat<()>>()
			.register_type::<RepeatTimes<()>>();
		#[cfg(feature = "scripting")]
		app.register_type::<ScriptLanguage>()
			.register_type::<Script<(), String>>();
	}
}
