use beet::prelude::*;

#[cfg(feature = "setup")]
fn main() {
	let cx = app_cx!();

	AppConfig::new()
		.add_step(BuildFileRoutes {
			files: "crates/beet_site/src/routes".into(),
			pkg_name: Some(cx.pkg_name.clone()),
			..Default::default()
		})
		// ensures design mockups are recollected on reload
		.add_step(beet::design::prelude::mockups())
		.export();
}


#[cfg(not(feature = "setup"))]
fn main() {
	AppRouter::new(app_cx!())
		// .add_collection(beet_site::prelude::routes::collect())
		// .add_plugin(Router::new)
		.run();
}
