//! Extension methods for Bevy's [`AppExit`] and cross-platform exit handling.

use crate::prelude::*;
use bevy::app::AppExit;
use bevy::ecs::schedule::common_conditions;

/// Plugin that exits the process upon an [`AppExit`] message.
///
/// Uses [`process_ext::exit`] for cross-platform compatibility.
#[derive(Default)]
pub struct AppExitPlugin;

impl Plugin for AppExitPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Last,
			cross_exit.run_if(common_conditions::on_message::<AppExit>),
		);
	}
}

fn cross_exit(mut app_ext: MessageReader<AppExit>) {
	if let Some(exit) = app_ext.read().next() {
		process_ext::exit(exit.exit_code());
	}
}

/// Extension trait adding utility methods to [`AppExit`].
#[extend::ext(name=AppExitExt)]
pub impl AppExit {
	/// Converts this [`AppExit`] into a [`Result`].
	fn into_result(self) -> Result {
		match self {
			AppExit::Success => Ok(()),
			AppExit::Error(err) => {
				bevybail!("Application exited with error: {}", err)
			}
		}
	}

	/// Returns the exit code for this [`AppExit`].
	fn exit_code(&self) -> i32 {
		match self {
			AppExit::Success => 0,
			AppExit::Error(code) => code.get() as i32,
		}
	}
}
