#[cfg(target_arch = "wasm32")]
fn main() {
	use beet_ml::wasm::open_or_fetch::open_or_fetch_blocking;
	console_log::init_with_level(log::Level::Info).ok();
	let result = open_or_fetch_blocking("https://png.pngtree.com/png-clipart/20200225/original/pngtree-image-of-cute-radish-vector-or-color-illustration-png-image_5274337.jpg");
	log::info!("SUCCESS {:?}", result);
}
#[cfg(not(target_arch = "wasm32"))]

fn main() {}
