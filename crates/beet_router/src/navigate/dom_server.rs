//! The browser's server facet: the DOM as a surface the served page's wasm
//! process paints, booted by `--server=dom`.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// The browser's server, the twin of [`TuiServer`](crate::prelude::TuiServer):
/// a facet on its entity's [`RunningSet`] whose `--server=dom` boots the page
/// into the document the process runs in.
///
/// The DOM is a surface. A page's `<Wasm server="dom"/>` boots the site's
/// entry in the tab through the same launch path as the terminal and the
/// server, and this facet spawns the [`DomHost`], the browser's page host,
/// with an in-world [`Navigator`] at the request path, so the browser
/// dispatches the url through the entry's own `RouteTree` exactly as the
/// terminal does. A launch naming no `--server=dom` boots the entry headless:
/// no host, nothing written to the document, the shape of a sensor or worker
/// process. Painting is a launch decision, never a build one.
///
/// Registered on every target so an entry keeps its shape wherever it loads,
/// but it boots only in a browser tab and refuses anywhere else, and
/// `default_boot` is off so a bare `beet` never selects it: the page that
/// serves the browser names it.
///
/// ```bsx
/// <CallOnReady {(HttpServer, TuiServer, DomServer, SweepDescendants)}>
/// ```
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
#[component(on_add = hook_ext::component_hook(DomServer::add_facet))]
pub struct DomServer {
	/// Whether a bare `beet` (no `--server`) boots this server. Off by default:
	/// the DOM is the browser's surface, and the page booting the browser
	/// process names it with `--server=dom`, so a native launch of the same
	/// entry never tries to paint a document it does not have.
	pub default_boot: bool,
}

impl Default for DomServer {
	fn default() -> Self {
		Self {
			default_boot: false,
		}
	}
}

/// The facet name a launch selects this server by.
const DOM: &str = "dom";

impl DomServer {
	/// This server's [`RunningSet`] facet: spawn the DOM host, hold it open
	/// until the shutdown signal, then despawn it.
	fn add_facet(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let default_boot = self.default_boot;
		move |entity: &mut EntityCommands| {
			RunningSet::<Request, Response>::add(
				entity,
				DOM,
				move |request: &Request| {
					RunningSetFilter::selects(
						request.params(),
						DOM,
						default_boot,
					)
				},
				|entity, request, shutdown| {
					let parts = request.request_parts().clone();
					Box::pin(serve_dom(entity, parts, shutdown))
				},
			);
		}
	}
}

/// The browser surface: the page bound to it is what the document shows.
///
/// A [`PageHost`] with its [`PageSlot`] and no buffer of its own, since its
/// paint target is the document body rather than a cell grid; the DOM
/// renderer walks the bound page into the body. Spawned as the [`DomServer`]'s
/// child so it goes with it, carrying the in-world [`Navigator`] that binds
/// the page.
#[derive(Debug, Default, Clone, Component)]
#[require(PageHost)]
#[component(on_add = hook_ext::observe(log_landing))]
pub struct DomHost;

impl DomHost {
	/// The host with its page slot, ready for a co-located [`Navigator`].
	pub fn bundle() -> impl Bundle { (DomHost, children![PageSlot]) }
}

/// Observer: the host's page landed, so the tab's console says which url the
/// world dispatched. The one line a boot check reads.
fn log_landing(ev: On<Insert, RenderSurfaceOf>, navigators: Query<&Navigator>) {
	if let Ok(navigator) = navigators.get(ev.entity) {
		info!("dom host landed {}", navigator.current_url());
	}
}

/// Boot the DOM host, hold it open, and despawn it once the shutdown signal
/// resolves.
async fn serve_dom(
	entity: AsyncEntity,
	parts: RequestParts,
	shutdown: OnceValueRx<()>,
) -> Result {
	assert_browser()?;
	// the opening route is the page's own url: the location's path is the
	// request path, so the world lands where the browser is.
	entity
		.insert(OpeningRoute::resolve(&entity, &parts).await?)
		.await?;
	let Some(host) = start_dom(entity.clone()).await? else {
		return Ok(());
	};
	shutdown.wait().await;
	entity
		.world()
		.with(move |world: &mut World| {
			// a despawned server takes its host with it; an interrupt keeps the
			// server and drops the host alone.
			world.try_despawn(host).ok();
		})
		.await;
	Ok(())
}

/// The DOM exists in a browser tab and nowhere else: a native launch (or a
/// deno/worker host) naming `--server=dom` is told what to do instead.
fn assert_browser() -> Result {
	#[cfg(target_arch = "wasm32")]
	let in_browser =
		js_runtime::environment() == js_runtime::JsEnvironment::Browser;
	#[cfg(not(target_arch = "wasm32"))]
	let in_browser = false;
	match in_browser {
		true => Ok(()),
		false => bevybail!(
			"`--server=dom` paints the document of a browser tab, which this \
			 process has none of: serve the entry (`--server=http`) and open \
			 its page, whose `<Wasm server=\"dom\"/>` boots this server there"
		),
	}
}

/// Wire the DOM host on `entity`'s router and return it, or [`None`] if the
/// server was despawned before it could boot.
async fn start_dom(entity: AsyncEntity) -> Result<Option<Entity>> {
	if !entity.is_alive().await {
		return Ok(None);
	}
	// navigation resolves routes against the url space's own `Router`, the
	// same hop the terminal server makes.
	let router = entity
		.world()
		.run_system_cached_with::<_, Result<Entity>, _, _>(
			find_router,
			entity.id(),
		)
		.await??;
	let home = entity.get(|route: &OpeningRoute| route.0.clone()).await?;
	// `<RoutesDir>` discovery runs a few ticks behind boot; settle it so the
	// opening route resolves on the first load. The served page is already
	// painted, so nothing shows a placeholder meanwhile.
	TemplatePending::settle(entity.world()).await;
	let server = entity.id();
	entity
		.world()
		.with(move |world: &mut World| {
			world
				.spawn((
					DomHost::bundle(),
					// the web target: the page negotiates html, so the layout
					// renders its document chrome rather than the terminal's
					Navigator::in_world(router, home)
						.with_accepts(vec![MediaType::Html]),
					ChildOf(server),
				))
				.id()
		})
		.await
		.xmap(Some)
		.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A server root with the DOM facet, booted with `args` and driven until
	/// its call resolves, returning the boot outcome.
	async fn boot(args: &str) -> Result<(), String> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, RouterPlugin));
		let root = app
			.world_mut()
			.spawn((DomServer::default(), children![Router::default()]))
			.id();
		let outcome = Store::<Option<Result<(), String>>>::default();
		let result = outcome.clone();
		app.world_mut()
			.entity_mut(root)
			.call_with(
				Request::from_cli_str(args),
				OutHandler::<Response>::new(move |_, out| {
					result.set(Some(
						out.map(|_| ()).map_err(|err| err.to_string()),
					));
					Ok(())
				}),
			)
			.unwrap();
		app_ext::update_until(&mut app, |_| outcome.get().is_some())
			.await
			.xpect_true();
		outcome.get().unwrap()
	}

	/// Outside a browser the facet refuses with guidance rather than parking
	/// a host nothing paints.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn refuses_outside_a_browser() {
		boot("--server=dom")
			.await
			.unwrap_err()
			.xpect_contains("browser tab");
	}

	/// A bare boot never selects the DOM: a native launch of an entry
	/// declaring it boots its other servers alone, here none at all.
	#[beet_core::test]
	async fn is_not_a_default_boot() {
		boot("")
			.await
			.unwrap_err()
			.xnot()
			.xpect_contains("browser tab");
	}
}
