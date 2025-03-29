use beet::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	let mut routes = beet_site::routes::collect();
	routes.extend(beet::design::mockups::collect().into_iter());
	AppRouter::new(app_cx!()).add_collection(routes).run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::wasm::collect())
		.run();
}
