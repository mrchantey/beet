use beet::prelude::*;

#[cfg(feature = "setup")]
fn main() {
	let cx = app_cx!();

	AppConfig::new()
		.add_step(BuildFileRoutes::new(
			cx.resolve_path("routes"),
			cx.resolve_path("codegen/routes.rs"),
			&cx.pkg_name,
		))
		// ensures design mockups are recollected on reload
		.add_step(beet::design::prelude::mockups_config())
		.add_wasm_step(BuildWasmRoutes::new(
			cx.resolve_path("codegen/wasm.rs"),
			&cx.pkg_name,
		))
		.export();
}


#[cfg(all(not(feature = "setup"), not(target_arch = "wasm32")))]
fn main() {
	let mut routes = beet_site::routes::collect();
	routes.extend(beet::design::mockups::collect().into_iter());
	AppRouter::new(app_cx!()).add_collection(routes).run();
}

#[cfg(all(not(feature = "setup"), target_arch = "wasm32"))]
fn main() {
	AppRouter::new(app_cx!())
		.add_collection(beet_site::wasm::collect())
		.run();
}
