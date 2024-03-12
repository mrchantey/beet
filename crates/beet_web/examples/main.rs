#[cfg(not(target_arch = "wasm32"))]
fn main() {}
#[cfg(target_arch = "wasm32")]
fn main() {
	use beet_web::app::AppOptions;
	AppOptions::from_url().run();
}
