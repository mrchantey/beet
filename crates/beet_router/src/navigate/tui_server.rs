//! The live-TUI server entry: boots a navigable terminal app on a router entity.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A live-TUI server owning the boot with its router as a child: the fan-out whose
/// `--server` selects `"tui"` boots the navigable terminal app. The interactive
/// sibling of the one-shot [`CliServer`].
///
/// A long-running server: it never resolves the boot call, so its
/// [`Running<Response>`](beet_action::prelude::Running) parks the process up. The
/// boot wires the live host: a [`StdioTerminal`] paired with a [`PageHost::bundle`]
/// buffer, plus an in-world [`Navigator`] pointed at this router, started at the
/// request path (`-- docs/design/form`, default home `/`). A
/// `--color-scheme=light|dark` argument seeds the app-wide [`Theme::scheme`], the
/// session's scheme on every page (layouts consult it). The app then runs
/// persistently, repainting reactively as navigation and input change the page;
/// the `CharcellTuiPlugin` loop drives it and Ctrl+c exits.
///
/// Reusable: any app gets a live TUI by adding the live plugins
/// ([`CharcellTuiPlugin`], [`NavigatorPlugin`], [`LivePagePlugin`]) and spreading
/// this on its server root, then booting it.
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
#[require(StartOnLoad)]
#[component(on_add = hook_ext::observe((on_action_in, on_running_removed)))]
pub struct TuiServer {
	/// Whether a bare `beet` (no `--server`) boots this server. `true` by default,
	/// so an entry declaring a single [`TuiServer`] needs no flag; clear it on a
	/// server that should boot only when `--server=tui` names it explicitly.
	pub default_boot: bool,
}

impl Default for TuiServer {
	fn default() -> Self { Self { default_boot: true } }
}

/// The live host entity (terminal + navigator) the boot spawned, despawned on
/// teardown so a reload does not leak it.
#[derive(Component)]
struct TuiHost(Entity);

/// Boots the live terminal app on the boot fan-out, if `--server` selects `"tui"`.
/// Records the opening route on the router (the shared mechanism the SSH server
/// also reads) and never resolves the boot call, so its `Running` parks the
/// process up.
fn on_action_in(
	ev: On<StartRunning<Request>>,
	servers: Query<&TuiServer>,
	mut commands: Commands,
) -> Result {
	let Ok(default_boot) = servers.get(ev.entity).map(|server| server.default_boot)
	else {
		return Ok(());
	};
	let (selected, opening, scheme) = ev.with(|request| {
		(
			// this server's own default unless `--server` names a set.
			Request::selects_server(request, "tui", default_boot),
			OpeningRoute::from_request(request),
			request
				.get_param("color-scheme")
				.and_then(ColorScheme::parse),
		)
	})?;
	if !selected {
		return Ok(());
	}
	commands
		.entity(ev.entity)
		// `ServerBooted` flags the boot as served, so `exit_if_no_server` lets it park
		.insert((ServerBooted, opening))
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
	// navigation targets the server entity; route lookups resolve the nearest
	// `RouteTree` by ancestry, so the server browses its router's routes.
	let router = entity.id();
	// the opening route is recorded on the server (the shared mechanism); read it
	// back here. The server is route-agnostic; a downstream plugin (eg
	// `CardStackPlugin`) may patch a more specific opening route after boot.
	let home = entity.get(|route: &OpeningRoute| route.0.clone()).await?;
	// the live host: a stdio terminal paired with the page-host buffer (rendered
	// together by `render_terminal`). Spawned with a "Loading…" placeholder and
	// *without* the navigator yet, so the first frames paint loading rather than a
	// blank screen. `--color-scheme` pins the session scheme app-wide.
	let host = entity
		.world()
		.with(move |world: &mut World| {
			if let Some(scheme) = scheme {
				world.get_resource_or_init::<Theme>().scheme = scheme;
			}
			let host = world
				.spawn((
					StdioTerminal::default(),
					PageHost::bundle(terminal_ext::size()),
				))
				.id();
			set_loading_page(world, host);
			host
		})
		.await;
	// `<RoutesDir>` discovery runs as an async task a few ticks behind boot, so the
	// opening route is not in the tree the instant the navigator loads it. Settle it
	// first so the home page resolves on the first load rather than flashing a
	// "no route matched /" error; the loading placeholder shows in the meantime.
	TemplatePending::settle(entity.world()).await;
	// now co-locate the in-world navigator on the host: its `on_add` browses this
	// router from `home`, binding the home page over the loading placeholder.
	entity
		.world()
		.with(move |world: &mut World| {
			world
				.entity_mut(host)
				.insert(Navigator::in_world(router, home));
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
