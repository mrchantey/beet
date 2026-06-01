//! The beet website binary.
//!
//! With the `codegen` feature it runs the route codegen pass and exits. With a
//! render target it boots the site router behind a server: an [`HttpServer`]
//! when the `web` feature is enabled, else a [`CliServer`] that renders a single
//! route to the terminal (ANSI) and exits.

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

/// The web target listens for HTTP requests.
#[cfg(all(not(feature = "codegen"), feature = "web"))]
fn site_server() -> impl beet::prelude::Bundle {
	beet::prelude::HttpServer::default()
}

/// The terminal-only target renders a single route to stdout and exits.
#[cfg(all(
	not(feature = "codegen"),
	not(feature = "web"),
	feature = "terminal"
))]
fn site_server() -> impl beet::prelude::Bundle { beet::prelude::CliServer }

#[cfg(not(any(feature = "codegen", feature = "render")))]
fn main() {
	panic!(
		"enable a render target (`web`/`terminal`) or the `codegen` feature"
	);
}
