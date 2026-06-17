//! The live-TUI server entry: boots a navigable terminal app on a router entity.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A live-TUI server: spawned alongside a router, a [`StartServer`] event whose
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
/// this on its router entity, then triggering a [`StartServer`].
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[component(on_add = on_add)]
pub struct TuiServer;

/// Registers the [`StartServer`] observer on the router, so the live terminal
/// app boots when a start event whose filter passes `"tui"` lands on it.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_start_server);
}

/// Boots the live terminal app when a [`StartServer`] passing `"tui"` lands.
/// A long-running server, so it inserts [`KeepAlive`].
fn on_start_server(ev: On<StartServer>, mut commands: Commands) {
	if !ev.passes("tui") {
		return;
	}
	commands.insert_resource(KeepAlive);
	// the trigger's params (eg the `beet serve` command's) carry the opening route
	// and scheme, so a served site opens at the right page rather than the serve
	// command's own argv.
	let params = ev.params.clone();
	commands
		.entity(ev.event_target())
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
	// A `path` param from the trigger (eg `beet serve <dir> --server=tui
	// --path=docs/design/form`) opens that route; absent it, the process argv (a
	// compiled binary's own args). `--color-scheme=light|dark` pins the session
	// scheme app-wide either way. The server is route-agnostic; a downstream plugin
	// (eg `CardStackPlugin`) may patch a more specific opening route after boot.
	let (home, scheme) = match params.get("path") {
		Some(path) => (
			Url::parse(path.as_str()),
			params.get("color-scheme").and_then(|s| ColorScheme::parse(s)),
		),
		None => {
			let request = Request::from_cli_args(CliArgs::parse_env());
			let scheme = request
				.get_param("color-scheme")
				.and_then(ColorScheme::parse);
			(Url::parse(request.path_string()), scheme)
		}
	};
	entity
		.world()
		.with(move |world: &mut World| {
			if let Some(scheme) = scheme {
				world.get_resource_or_init::<Theme>().scheme = scheme;
			}
			// the live host: a stdio terminal paired with the page-host buffer,
			// rendered together by `render_terminal` (one entity, both components).
			world.spawn((
				StdioTerminal::default(),
				page_host(terminal_ext::size()),
			));
			// an in-world navigator browsing this router, starting at `home`
			world.spawn(Navigator::in_world(router, home));
		})
		.await;
	Ok(())
}
