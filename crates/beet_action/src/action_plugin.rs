use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers action control-flow types for world serialization.
///
/// Registers the unit-input instantiations of control-flow components
/// so that action trees can be serialized and deserialized.
#[derive(Default)]
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<MinimalTypesPlugin>()
			// async actions queue work onto the AsyncWorld; without this
			// plugin the runtime never drives them
			.init_plugin::<AsyncPlugin>()
			// control-flow types
			.register_type::<ChildError>()
			.register_type::<CallOnSpawn<(), Outcome>>()
			.add_systems(Update, call_on_spawn::<(), Outcome>)
			.register_type::<ExcludeErrors>()
			.register_type::<Sequence<(), ()>>()
			.register_type::<InfallibleSequence<(), ()>>()
			.register_type::<Parallel<(), ()>>()
			.register_type::<HighestScore<(), ()>>()
			.register_type::<Score>()
			.register_type::<Repeat<()>>()
			.register_type::<RepeatTimes<()>>()
			// agent resolution types
			.register_type::<ActionOf>()
			.register_type::<Actions>()
			.register_type::<TargetEntity>()
			// leaf / util actions
			.register_type::<EndWith<Outcome>>()
			.register_type::<Log>()
			.register_type::<SucceedTimes>()
			.register_type::<RunTimer>()
			.register_type::<RunNext>()
			.register_type::<NoInterrupt>()
			// long-running action lifecycle
			.add_systems(Update, tick_run_timers)
			.add_plugins(running_plugin::<(), Outcome>);
		#[cfg(feature = "scripting")]
		app.register_type::<ScriptLanguage>()
			.register_type::<Script<(), String>>();
		// the `StartScript` load verb, so a `<script {StartScript}>` entry resolves
		// it. Native-only, like its quickjs console runtime.
		#[cfg(all(feature = "quickjs", feature = "json", not(target_arch = "wasm32")))]
		app.register_type::<StartScript>();
	}
}
