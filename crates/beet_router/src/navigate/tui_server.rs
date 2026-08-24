//! The live-TUI server entry: boots a navigable terminal app on a router entity.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A live-TUI server adding a facet to its entity's [`RunningSet`] with its
/// router as a child: the facet whose `--server` selects `"tui"` boots the
/// navigable terminal app. The interactive sibling of the one-shot [`CliServer`].
///
/// A long-running server: it never resolves the parked call, so the entity's
/// [`Running<Response>`](beet_action::prelude::Running) parks the process up. The
/// facet wires the live host: a [`StdioTerminal`] paired with a [`PageHost::bundle`]
/// buffer, plus an in-world [`Navigator`] pointed at this router, started at the
/// request path (`-- docs/design/form`, default home `/`). A
/// `--color-scheme=light|dark` argument seeds the app-wide [`Theme::scheme`], the
/// session's scheme on every page (layouts consult it). The app then runs
/// persistently, repainting reactively as navigation and input change the page;
/// the `CharcellTuiPlugin` loop drives it and Ctrl+c exits. On the shutdown
/// signal it despawns that host, so the terminal does not outlive the run.
///
/// Reusable: any app gets a live TUI by adding the live plugins
/// ([`CharcellTuiPlugin`], [`NavigatorPlugin`], [`LivePagePlugin`]) and spreading
/// this on its server root, then booting it.
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
#[component(on_add = hook_ext::component_hook(TuiServer::add_facet))]
pub struct TuiServer {
	/// Whether a bare `beet` (no `--server`) boots this server. `true` by default,
	/// so an entry declaring a single [`TuiServer`] needs no flag; clear it on a
	/// server that should boot only when `--server=tui` names it explicitly.
	pub default_boot: bool,
}

impl Default for TuiServer {
	fn default() -> Self { Self { default_boot: true } }
}

impl TuiServer {
	/// This server's [`RunningSet`] facet: spawn the live host, hold it open until
	/// the shutdown signal, then despawn it.
	fn add_facet(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		// selection is read once here, when the server is declared, so the facet
		// decides without a world access.
		let default_boot = self.default_boot;
		move |entity: &mut EntityCommands| {
			RunningSet::<Request, Response>::add(
				entity,
				"tui",
				move |request: &Request| {
					RunningSetFilter::selects(
						request.params(),
						"tui",
						default_boot,
					)
				},
				|entity, request, shutdown| {
					// the future owns what it needs, so nothing borrows the input
					// past this call.
					let opening = OpeningRoute::from_request(request);
					let scheme = request
						.get_param("color-scheme")
						.and_then(ColorScheme::parse);
					Box::pin(serve_tui(entity, opening, scheme, shutdown))
				},
			);
		}
	}
}

/// Boot the live terminal app, hold it open, and despawn it once the shutdown
/// signal resolves, so its terminal and navigator do not leak past a reload, an
/// interrupt or a despawn.
///
/// The host id is a local rather than a slot on the server: one future owns the
/// whole lifecycle, so a teardown driven by the server's own despawn still takes
/// the terminal with it.
async fn serve_tui(
	entity: AsyncEntity,
	opening: Result<OpeningRoute>,
	scheme: Option<ColorScheme>,
	shutdown: OnceValueRx<()>,
) -> Result {
	// the opening route is recorded on the server (the shared mechanism the SSH
	// server also reads).
	entity.insert(opening?).await?;
	let Some(host) = start_tui(entity.clone(), scheme).await? else {
		return Ok(());
	};
	shutdown.wait().await;
	entity
		.world()
		.with(move |world: &mut World| {
			// already gone when the whole scene tore down, which is a teardown just
			// the same
			world.try_despawn(host).ok();
		})
		.await;
	Ok(())
}

/// Wire the live host on `entity`'s router and return it, or [`None`] if the
/// server was despawned before it could boot.
async fn start_tui(
	entity: AsyncEntity,
	scheme: Option<ColorScheme>,
) -> Result<Option<Entity>> {
	// a briefly-spawned server (eg during serialization) has no business booting
	if !entity.is_alive().await {
		return Ok(None);
	}
	// navigation resolves routes against a `RouteTree`, which lives on the url
	// space's own `Router` rather than on the server that hosts it, so address
	// the router itself (the same hop `exchange_child` makes).
	let router = entity
		.world()
		.run_system_cached_with::<_, Result<Entity>, _, _>(
			find_router,
			entity.id(),
		)
		.await??;
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
	Ok(Some(host))
}
