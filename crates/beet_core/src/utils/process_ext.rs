use crate::prelude::*;

/// Cross-platform process exit
/// - Native: calls `std::process::exit`
/// - Wasm: calls `js_runtime::exit`
pub fn exit(exit_code: i32) {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			js_runtime::exit(exit_code);
		} else {
			std::process::exit(exit_code);
		}
	}
}
