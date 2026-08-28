//! The interactive REPL server: a facet reading lines from stdin, dispatching
//! each through the host's dispatch child and streaming the response to stdout.
use super::call_on_ready::stream_body_to_stdout;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::futures_lite;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Where a [`ReplServer`] reads its lines: a factory called once per start,
/// returning the channel the loop drains.
///
/// A closure so a test (or an embedding host) drives the loop without a
/// terminal, exactly as [`HttpServerBackend`] lets a test serve without a socket.
pub type ReplLineSource =
	Arc<dyn Fn() -> async_channel::Receiver<String> + Send + Sync>;

/// An interactive REPL (read-eval-print loop) server adding a facet to its
/// entity's [`RunningSet`], with its dispatch host as a child: the facet whose
/// `--server` selects `"repl"` renders the start request, then dispatches each
/// line typed at the prompt.
///
/// The interactive sibling of the one-shot [`CliServer`]: same routes, same
/// request shape, held open across many lines instead of exiting after one.
///
/// ```bsx
/// <ReplServer>
///     <Router>..</Router>
/// </ReplServer>
/// ```
///
/// Each non-empty line is parsed as CLI-style arguments into a [`Request`],
/// dispatched through the host's dispatch child ([`exchange_child`]) and streamed
/// to stdout, followed by that route's `--help` so the prompt always shows what
/// else is reachable. Typing `exit` or `quit`, or an EOF on stdin, resolves the
/// parked call ([`EndRun`]) so the process exits zero like any other completed
/// run. Pair it with a [`History`] component to track the current path, enabling
/// relative navigation via `--navigate=<direction>`.
///
/// [`exchange_child`]: AsyncExchangeExt::exchange_child
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
#[component(on_add = hook_ext::component_hook(ReplServer::add_facet))]
pub struct ReplServer {
	/// Whether a bare `beet` (no `--server`) boots this server. `true` by default,
	/// so an entry declaring a single [`ReplServer`] needs no flag; clear it on an
	/// entry whose repl should boot only when `--server=repl` names it, ie one
	/// that also declares a [`CliServer`].
	pub default_boot: bool,
	/// This server's own line source, replacing the stdin reader. `None` (the
	/// default) reads stdin.
	///
	/// Not markup-declarable (it holds a closure), so it is set by whoever spawns
	/// the entity: a test driving the loop, or a host that already owns the
	/// terminal.
	#[reflect(ignore)]
	pub lines: Option<ReplLineSource>,
}

impl Default for ReplServer {
	fn default() -> Self {
		Self {
			default_boot: true,
			lines: None,
		}
	}
}

impl ReplServer {
	/// Read lines from `lines` rather than stdin, so this one repl owns where its
	/// input comes from.
	pub fn with_lines<Func>(mut self, lines: Func) -> Self
	where
		Func: 'static + Send + Sync + Fn() -> async_channel::Receiver<String>,
	{
		self.lines = Some(Arc::new(lines));
		self
	}

	/// This server's [`RunningSet`] facet: render the start request, then hold the
	/// prompt open until stdin ends or the shutdown signal resolves.
	fn add_facet(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		// selection and line source are read once here, when the server is
		// declared, so the facet decides without a world access.
		let default_boot = self.default_boot;
		let lines = self.lines.clone();
		move |entity: &mut EntityCommands| {
			RunningSet::<Request, Response>::add(
				entity,
				"repl",
				move |request: &Request| {
					RunningSetFilter::selects(
						request.params(),
						"repl",
						default_boot,
					)
				},
				move |entity, request, shutdown| {
					// the future owns what it needs, so nothing borrows the input
					// past this call; a start request is argv-shaped and carries no
					// body, so cloning the parts is the whole copy.
					let parts = request.request_parts().clone();
					let lines = match &lines {
						Some(source) => source(),
						None => stdin_lines(),
					};
					Box::pin(serve_repl(entity, parts, lines, shutdown))
				},
			);
		}
	}
}

/// Hold the prompt open, racing the loop against the shutdown signal so a
/// teardown (an interrupt, a reload, a co-resident facet failing) drops the
/// reader rather than leaving it draining a channel nobody serves.
async fn serve_repl(
	entity: AsyncEntity,
	parts: RequestParts,
	lines: async_channel::Receiver<String>,
	shutdown: OnceValueRx<()>,
) -> Result {
	futures_lite::future::or(repl_loop(entity, parts, lines), async move {
		shutdown.wait().await;
		Result::Ok(())
	})
	.await
}

/// Render the start request, then dispatch every line typed at the prompt until
/// stdin ends.
///
/// The start request is the first command, so `--server=repl about` opens on that
/// page and a bare start renders the root.
async fn repl_loop(
	entity: AsyncEntity,
	parts: RequestParts,
	lines: async_channel::Receiver<String>,
) -> Result {
	dispatch(&entity, Request::from_parts(parts, default())).await?;
	cross_log_noline!("> ");
	while let Ok(line) = lines.recv().await {
		let line = line.trim();
		if line == "exit" || line == "quit" {
			break;
		}
		dispatch(&entity, Request::from_cli_str(line)).await?;
		cross_log_noline!("> ");
	}
	// stdin closed, or the user asked to leave: the repl IS the run, so ending it
	// resolves the parked call and the process exits like any completed one.
	entity.queue(EndRun(Response::ok())).await?
}

/// Dispatch one request, then repeat it with `--help` so each render lists what
/// else the current route offers. A request already asking for help renders once.
async fn dispatch(entity: &AsyncEntity, request: Request) -> Result {
	let help = (!request.has_param("help")).then(|| {
		Request::from_parts(
			request.parts().clone().with_flag("help"),
			default(),
		)
	});
	render(entity, request).await?;
	if let Some(help) = help {
		cross_log!("");
		render(entity, help).await?;
	}
	Ok(())
}

/// Dispatch through the host's dispatch child and stream the body to stdout.
///
/// A server holds no dispatch of its own, so this is the same downward hop
/// [`CliServer`] makes; the repl differs only in making it many times.
async fn render(entity: &AsyncEntity, request: Request) -> Result {
	let accept = vec![MediaType::AnsiTerm, MediaType::Markdown];
	let response = entity
		.exchange_child(request.with_header::<header::Accept>(accept))
		.await;
	let (parts, body) = response.into_parts();
	stream_body_to_stdout(body).await?;
	cross_log!("");
	if parts.status().is_err() {
		error!("command failed\nStatus: {}", parts.status());
	}
	Ok(())
}

/// Read stdin on a background thread, so the async executor is never blocked.
///
/// The thread ends when stdin closes or the receiver is dropped (the facet's
/// teardown). A thread already parked in `read_line` cannot be interrupted, so it
/// dies with the process: the standard stdin caveat, not a leaked task.
fn stdin_lines() -> async_channel::Receiver<String> {
	let (tx, rx) = async_channel::unbounded::<String>();
	std::thread::spawn(move || {
		let stdin = std::io::stdin();
		loop {
			let mut line = String::new();
			match stdin.read_line(&mut line) {
				Ok(0) => break, // EOF
				Ok(_) => {
					if tx.send_blocking(line).is_err() {
						break;
					}
				}
				Err(_) => break,
			}
		}
	});
	rx
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::exports::async_channel;
	use beet_core::prelude::*;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		app
	}

	/// A repl fed from `lines`, with one dispatch child echoing the request path
	/// so a test can see which requests reached it.
	fn spawn_repl(
		app: &mut App,
		lines: async_channel::Receiver<String>,
		log: Store<Vec<String>>,
	) -> Entity {
		app.world_mut()
			.spawn((
				ReplServer::default().with_lines(move || lines.clone()),
				children![exchange_ext::handler(move |cx| {
					log.push(cx.path_string());
					Response::ok()
				})],
			))
			.id()
	}

	/// Call the repl's action and let the result go: a live repl parks the call,
	/// and a declined start fails it, so neither outcome is the assertion.
	fn call_and_park(app: &mut App, entity: Entity, request: Request) {
		app.world_mut().entity_mut(entity).run_async_local(
			move |server| async move {
				server.call::<Request, Response>(request).await.ok();
				Ok(())
			},
		);
	}

	/// A bare start selects the repl: the start request renders through the
	/// dispatch child, and the run parks on the prompt.
	#[beet_core::test]
	async fn dispatches_through_the_child() {
		let log = Store::<Vec<String>>::default();
		let (send, recv) = async_channel::unbounded::<String>();
		let mut app = app();
		let entity = spawn_repl(&mut app, recv, log);
		call_and_park(&mut app, entity, Request::from_cli_str(""));
		// the start request, then its `--help` twin
		app_ext::update_until(&mut app, |_| log.len() >= 2)
			.await
			.xpect_true();
		send.send("about".to_string()).await.unwrap();
		app_ext::update_until(&mut app, |_| log.len() >= 4)
			.await
			.xpect_true();
		log.get()[2].xref().xpect_eq("/about");
		// a live repl holds the run open
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
	}

	/// An EOF on the line source resolves the parked call rather than writing
	/// `AppExit` behind the run's back, so the process exits like any other
	/// completed one.
	#[beet_core::test]
	async fn eof_ends_the_run() {
		let log = Store::<Vec<String>>::default();
		let (send, recv) = async_channel::unbounded::<String>();
		let mut app = app();
		let entity = spawn_repl(&mut app, recv, log);
		call_and_park(&mut app, entity, Request::from_cli_str(""));
		app_ext::update_until(&mut app, |_| log.len() >= 2)
			.await
			.xpect_true();
		drop(send);
		app_ext::update_until(&mut app, |world| {
			!world.entity(entity).contains::<Running<Response>>()
		})
		.await
		.xpect_true();
	}

	/// A start whose `--server` does not select `"repl"` leaves it dormant: the
	/// lone declining facet starts nothing, so the call fails instead of parking.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		let log = Store::<Vec<String>>::default();
		let (_send, recv) = async_channel::unbounded::<String>();
		let mut app = app();
		let entity = spawn_repl(&mut app, recv, log);
		call_and_park(&mut app, entity, Request::from_cli_str("--server=cli"));
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		log.get().xpect_eq(Vec::<String>::new());
	}
}
