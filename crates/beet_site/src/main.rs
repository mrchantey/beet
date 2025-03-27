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
		.add_step(beet::design::prelude::mockups())
		.add_wasm_step(BuildWasmRoutes::new(
			cx.resolve_path("codegen/routes_wasm.rs"),
			&cx.pkg_name,
		))
		.export();
}


#[cfg(not(feature = "setup"))]
fn main() {
	AppRouter::new(app_cx!())
		// .add_collection(beet_site::prelude::routes::collect())
		// .add_plugin(Router::new)
		.run();
}
