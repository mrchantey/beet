#[cfg(not(target_arch = "wasm32"))]
fn main() {}
#[cfg(target_arch = "wasm32")]
fn main() {
	use beet_web::prelude::*;
	use forky_core::ResultTEExt;
	DomSim {
		flowers: 10,
		..Default::default()
	}
	.with_url_params::<BeeNode>()
	.run_forever()
	.ok_or(|e| log::error!("{e}"));
}
