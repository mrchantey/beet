use crate::bevybail;
use bevy::app::AppExit;

#[extend::ext(name=AppExitExt)]
pub impl AppExit {
	fn into_result(self) -> bevy::prelude::Result {
		match self {
			AppExit::Success => Ok(()),
			AppExit::Error(err) => {
				bevybail!("Application exited with error: {}", err)
			}
		}
	}

	fn exit_code(&self) -> i32 {
		match self {
			AppExit::Success => 0,
			AppExit::Error(code) => code.get() as i32,
		}
	}

	/// Exit the application with the given exit code.
	fn into_exit(self) {
		#[cfg(not(target_arch = "wasm32"))]
		std::process::exit(self.exit_code());
		#[cfg(target_arch = "wasm32")]
		crate::js_runtime::exit(self.exit_code());
	}
}
