#[cfg(not(target_arch = "wasm32"))]
fn main() {}
#[cfg(target_arch = "wasm32")]
fn main() {
	use beet_web::prelude::*;
	use forky_core::ResultTEExt;
	DomSim::<BeetWebNode> {
		// flowers: 10,
		..Default::default()
	}
	.with_test_container(render_container())
	.with_url_params()
	.run_forever()
	.ok_or(|e| log::error!("{e}"));
}
