#[cfg(not(target_arch = "wasm32"))]
fn main() {}
#[cfg(target_arch = "wasm32")]
fn main() {
	use beet_web::prelude::*;
	use forky_core::ResultTEExt;
	BeetWebApp::default()
		.with_test_container()
		.with(spawn_bee)
		.with_bundle(flower_bundle())
		.run_forever()
		.ok_or(|e| log::error!("{e}"));
}
