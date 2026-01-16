/// Cross-platform process exit
/// - Native: calls `std::process::exit`
/// - Wasm: calls `js_runtime::exit`
pub fn exit(exit_code: i32) {
	#[cfg(not(target_arch = "wasm32"))]
	{
		std::process::exit(exit_code);
	}
	#[cfg(target_arch = "wasm32")]
	{
		use crate::prelude::*;
		js_runtime::exit(exit_code);
	}
}
