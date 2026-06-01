//! The beet website binary.
//!
//! With the `codegen` feature it runs the route codegen pass and exits. With a
//! render target (`web` and/or `terminal`) it boots the site server.

#[cfg(feature = "codegen")]
fn main() -> beet::prelude::Result {
	beet_site::run_codegen()
}

#[cfg(all(not(feature = "codegen"), feature = "render"))]
fn main() {
	use beet::prelude::*;
	use beet_site::prelude::*;

	App::new()
		.add_plugins((server_plugin, BeetPlugins))
		.insert_resource(PackageConfig {
			title: "Beet".to_string(),
			..pkg_config!()
		})
		.run();
}

#[cfg(not(any(feature = "codegen", feature = "render")))]
fn main() {
	panic!("enable a render target (`web`/`terminal`) or the `codegen` feature");
}
