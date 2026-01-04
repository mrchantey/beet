use crate::prelude::*;

/// Inhibit exiting wasm on app exit, useful for nested apps in tests
#[derive(Default, Resource)]
pub struct NoExitWasm;

/// Ensures the binary exits upon receiving an [`AppExit`]
/// in the case its being run by a [`js_runtime`](crate::js_runtime)
#[derive(Default)]
pub struct JsRuntimePlugin;

impl Plugin for JsRuntimePlugin {
	#[allow(unused)]
	fn build(&self, app: &mut App) {
		#[cfg(target_arch = "wasm32")]
		app.add_systems(
			PostUpdate,
			exit_wasm.run_if(|world: &World| {
				!world.contains_resource::<NoExitWasm>()
			}),
		);
	}
}

/// System that exits the wasm runtime with the given exit code,
/// upon receiving an [`AppExit`] message
#[cfg(target_arch = "wasm32")]
fn exit_wasm(mut reader: MessageReader<AppExit>) {
	for exit in reader.read() {
		crate::js_runtime::exit(exit.exit_code());
	}
}
