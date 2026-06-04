//! The beet website binary.
//!
//! With the `codegen` feature it runs the route codegen pass and exits.
//! Otherwise it boots the site router behind a server: an [`HttpServer`] by
//! default, or a [`CliServer`] (with the `cli` feature, or when no `web` target
//! is enabled) that renders a single route to stdout (HTML or ANSI per
//! `--accept`) and exits.

#[cfg(feature = "codegen")]
fn main() -> beet::prelude::Result { beet_site::run_codegen() }

#[cfg(all(not(feature = "codegen"), feature = "render"))]
fn main() {
	use beet::prelude::*;
	use beet_site::prelude::*;

	App::new()
		.add_plugins(server_plugin)
		.insert_resource(PackageConfig {
			title: "Beet".to_string(),
			..pkg_config!()
		})
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((site_server(), beet_site_router()));
		})
		.run();
}

/// Boots an HTTP server, the default web target.
#[cfg(all(not(feature = "codegen"), feature = "web", not(feature = "cli")))]
fn site_server() -> impl beet::prelude::Bundle {
	beet::prelude::HttpServer::default()
}

/// Renders a single route to stdout and exits. Selected by the `cli` feature, or
/// whenever no `web` target is present (eg `--no-default-features --features
/// terminal`).
#[cfg(all(
	not(feature = "codegen"),
	any(feature = "cli", not(feature = "web"))
))]
fn site_server() -> impl beet::prelude::Bundle { beet::prelude::CliServer }

#[cfg(not(any(feature = "codegen", feature = "render")))]
fn main() {
	panic!(
		"enable a render target (`web`/`terminal`) or the `codegen` feature"
	);
}
