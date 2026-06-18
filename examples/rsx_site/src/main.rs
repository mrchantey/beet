//! The rsx_site binary.
//!
//! With the `codegen` feature it runs the route codegen pass and exits.
//! Otherwise it spawns the site router with a server selected by build features,
//! then triggers a [`StartServer`] on it (empty filter, so whichever server is
//! present boots). The server is an [`HttpServer`] by default, the live
//! `TuiServer` under `tui`, or a [`CliServer`] (with the `cli` feature, or when
//! no `web` target is enabled) that renders a single route to stdout (HTML or
//! ANSI per `--accept`) and exits.

#[cfg(feature = "codegen")]
fn main() -> beet::prelude::Result { rsx_site::run_codegen() }

#[cfg(all(not(feature = "codegen"), feature = "render"))]
fn main() {
	use beet::prelude::*;
	use rsx_site::prelude::*;

	let mut app = App::new();
	app.add_plugins(server_plugin).insert_resource(PackageConfig {
		title: "Beet".into(),
		..pkg_config!()
	});
	// the live TUI layers the interactive plugins onto the shared substrate:
	// the charcell host loop, link navigation, and the current-page painter.
	#[cfg(feature = "tui")]
	app.add_plugins((CharcellTuiPlugin, NavigatorPlugin, LivePagePlugin));
	app.add_systems(Startup, |mut commands: Commands| {
		// spawn the site host first (registering its server's `on_add` observers),
		// then trigger the start: the empty filter matches whichever server the
		// build feature selected.
		commands
			.spawn((site_server(), rsx_site_router()))
			.trigger(StartServer::all);
	});
	app.run();
}

/// Boots the navigable live TUI, the interactive terminal target. Wins over the
/// `web`/`cli` arms when enabled (it is layered onto the default features).
#[cfg(all(not(feature = "codegen"), feature = "tui"))]
fn site_server() -> impl beet::prelude::Bundle { beet::prelude::TuiServer }

/// Boots an HTTP server, the default web target.
#[cfg(all(
	not(feature = "codegen"),
	feature = "web",
	not(feature = "cli"),
	not(feature = "tui")
))]
fn site_server() -> impl beet::prelude::Bundle {
	beet::prelude::HttpServer::default()
}

/// Renders a single route to stdout and exits. Selected by the `cli` feature, or
/// whenever no `web` target is present (eg `--no-default-features --features
/// terminal`).
#[cfg(all(
	not(feature = "codegen"),
	any(feature = "cli", not(feature = "web")),
	not(feature = "tui")
))]
fn site_server() -> impl beet::prelude::Bundle { beet::prelude::CliServer }

#[cfg(not(any(feature = "codegen", feature = "render")))]
fn main() {
	panic!(
		"enable a render target (`web`/`terminal`) or the `codegen` feature"
	);
}
