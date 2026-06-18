//! The live-TUI server entry: boots a navigable terminal app on a router entity.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A live-TUI server: spawned alongside a router, a [`BootServer`] event whose
/// filter passes `"tui"` boots the navigable terminal app. The interactive
/// sibling of the one-shot [`CliServer`].
///
/// A long-running server: starting it inserts [`KeepAlive`] so the process
/// persists. The start wires the live host: a [`StdioTerminal`] paired with a
/// [`page_host`] buffer, plus an in-world [`Navigator`] pointed at this router,
/// started at the CLI path argument (`-- docs/design/form`, default home `/`). A
/// `--color-scheme=light|dark` argument seeds the app-wide [`Theme::scheme`],
/// the session's scheme on every page (layouts consult it). The app
/// then runs persistently, repainting reactively as navigation and input change
/// the page; the `CharcellTuiPlugin` loop drives it and Ctrl+c exits.
///
/// Reusable: any app gets a live TUI by adding the live plugins
/// ([`CharcellTuiPlugin`], [`NavigatorPlugin`], [`LivePagePlugin`]) and spawning
/// this on its router entity, then triggering a [`BootServer`].
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[component(on_add = on_add)]
pub struct TuiServer;

/// Registers the [`BootServer`] observer on the router, so the live terminal
/// app boots when a start event whose filter passes `"tui"` lands on it.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_start_server);
}

/// Boots the live terminal app when a [`BootServer`] passing `"tui"` lands.
/// A long-running server, so it inserts [`KeepAlive`].
fn on_start_server(ev: On<BootServer>, mut commands: Commands) {
	if !ev.passes("tui") {
		return;
	}
	commands.insert_resource(KeepAlive);
	// the boot event's params (the `StartServer` verb's entry request, or a `beet
	// serve` command's argv) carry the opening route and scheme, so a served site
	// opens at the right page. Record the opening route on the router (the shared
	// mechanism the SSH server also reads); the scheme is applied at boot.
	let params = ev.params.clone();
	commands
		.entity(ev.event_target())
		.insert(OpeningRoute::from_params(&params))
		.queue_async_local(move |entity| boot(entity, params));
}

async fn boot(
	entity: AsyncEntity,
	params: MultiMap<SmolStr, SmolStr>,
) -> Result {
	// a briefly-spawned server (eg during serialization) has no business booting
	if !entity.is_alive().await {
		return Ok(());
	}
	let router = entity.id();
	// the opening route is recorded on the router (the shared mechanism); read it
	// back here. The server is route-agnostic; a downstream plugin (eg
	// `CardStackPlugin`) may patch a more specific opening route after boot.
	let home = entity.get(|route: &OpeningRoute| route.0.clone()).await?;
	// `--color-scheme=light|dark` pins the session scheme app-wide (the single
	// local surface): from the trigger params, else the process argv.
	let scheme = match params.get("color-scheme") {
		Some(scheme) => ColorScheme::parse(scheme),
		None => Request::from_cli_args(CliArgs::parse_env())
			.get_param("color-scheme")
			.and_then(ColorScheme::parse),
	};
	entity
		.world()
		.with(move |world: &mut World| {
			if let Some(scheme) = scheme {
				world.get_resource_or_init::<Theme>().scheme = scheme;
			}
			// the live host: a stdio terminal paired with the page-host buffer
			// (rendered together by `render_terminal`) plus the in-world navigator
			// co-located on it, browsing this router from `home`.
			world.spawn((
				StdioTerminal::default(),
				page_host(terminal_ext::size()),
				Navigator::in_world(router, home),
			));
		})
		.await;
	Ok(())
}
