//! The live-TUI server entry: boots a navigable terminal app on a router entity.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A live-TUI server: spread alongside a router, the boot fan-out whose
/// `--server` selects `"tui"` boots the navigable terminal app. The interactive
/// sibling of the one-shot [`CliServer`].
///
/// A long-running server: it never resolves the boot call, so the host's
/// [`Running<Response>`](beet_action::prelude::Running) parks the process up. The
/// boot wires the live host: a [`StdioTerminal`] paired with a [`page_host`]
/// buffer, plus an in-world [`Navigator`] pointed at this router, started at the
/// request path (`-- docs/design/form`, default home `/`). A
/// `--color-scheme=light|dark` argument seeds the app-wide [`Theme::scheme`], the
/// session's scheme on every page (layouts consult it). The app then runs
/// persistently, repainting reactively as navigation and input change the page;
/// the `CharcellTuiPlugin` loop drives it and Ctrl+c exits.
///
/// Reusable: any app gets a live TUI by adding the live plugins
/// ([`CharcellTuiPlugin`], [`NavigatorPlugin`], [`LivePagePlugin`]) and spreading
/// this on its router entity, then booting it.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun<Boot, Response>)]
#[component(on_add = on_add)]
pub struct TuiServer;

/// The live host entity (terminal + navigator) the boot spawned, despawned on
/// teardown so a reload does not leak it.
#[derive(Component)]
struct TuiHost(Entity);

/// Registers the boot ([`StartRunning<Boot>`]) and teardown
/// (`On<Remove, Running<Response>>`) observers on the router.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_action_in)
		.observe_any(on_running_removed);
}

/// Boots the live terminal app on the boot fan-out, if `--server` selects `"tui"`.
/// Records the opening route on the router (the shared mechanism the SSH server
/// also reads) and never resolves the boot call, so its `Running` parks the
/// process up.
fn on_action_in(ev: On<StartRunning<Boot>>, mut commands: Commands) -> Result {
	let (selected, opening, scheme) = ev.with(|boot| {
		(
			request_selects_server(boot, "tui"),
			OpeningRoute::from_request(boot),
			boot.get_param("color-scheme").and_then(ColorScheme::parse),
		)
	})?;
	if !selected {
		return Ok(());
	}
	commands
		.entity(ev.entity)
		.insert(opening)
		.queue_async_local(move |entity| start_tui(entity, scheme));
	Ok(())
}

/// Tears down the live terminal app when the host's `Running<Response>` is removed
/// (a reload, interrupt, or despawn): despawns the spawned host so its terminal
/// and navigator do not leak.
fn on_running_removed(
	ev: On<Remove, Running<Response>>,
	hosts: Query<&TuiHost>,
	mut commands: Commands,
) {
	if let Ok(host) = hosts.get(ev.event().event_target()) {
		commands.entity(host.0).try_despawn();
	}
}

async fn start_tui(entity: AsyncEntity, scheme: Option<ColorScheme>) -> Result {
	// a briefly-spawned server (eg during serialization) has no business booting
	if !entity.is_alive().await {
		return Ok(());
	}
	let router = entity.id();
	// the opening route is recorded on the router (the shared mechanism); read it
	// back here. The server is route-agnostic; a downstream plugin (eg
	// `CardStackPlugin`) may patch a more specific opening route after boot.
	let home = entity.get(|route: &OpeningRoute| route.0.clone()).await?;
	// the live host: a stdio terminal paired with the page-host buffer (rendered
	// together by `render_terminal`) plus the in-world navigator co-located on it,
	// browsing this router from `home`. `--color-scheme` pins the session scheme
	// app-wide (the single local surface).
	let host = entity
		.world()
		.with(move |world: &mut World| {
			if let Some(scheme) = scheme {
				world.get_resource_or_init::<Theme>().scheme = scheme;
			}
			world
				.spawn((
					StdioTerminal::default(),
					page_host(terminal_ext::size()),
					Navigator::in_world(router, home),
				))
				.id()
		})
		.await;
	// record the host so teardown can despawn it
	entity
		.with(move |mut entity| {
			entity.insert(TuiHost(host));
		})
		.await
		.ok();
	Ok(())
}
