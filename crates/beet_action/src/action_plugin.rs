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
			.register_type::<Fallback<(), ()>>()
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
			// the environment-agnostic drive leaf, the `DifferentialDrive` command it
			// writes onto a driven body (via `AgentQuery`), and the typed velocity
			// units its markup fields coerce from.
			.register_type::<SetDrive>()
			.register_type::<SetDriveAction>()
			.register_type::<DifferentialDrive>()
			.register_type::<DriveForDuration>()
			.register_type::<LinearVelocity>()
			.register_type::<AngularVelocity>()
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
		// the external-process leaf needs the native `ChildProcess` to spawn, so it
		// (and its action marker) only exist on a native std build. `Command` is
		// crate-qualified to disambiguate it from bevy's `Command` trait, both in
		// scope here via glob.
		#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::Command>()
			.register_type::<CommandAction>();
	}
}
