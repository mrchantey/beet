//! In-memory HTTP server: transports [`Request`]/[`Response`] over an
//! [`async_channel`] instead of a socket, sharing all of [`HttpServer`]'s
//! boot/park/dispatch machinery.
use crate::prelude::*;
use async_channel::Receiver;
use async_channel::Sender;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A self-contained HTTP server that reads [`Request`]s from a channel and writes
/// [`Response`]s back, sharing all of [`HttpServer`]'s boot/park/dispatch
/// machinery without binding a socket.
///
/// Unlike the singular [`set_http_server`] backend (the one bind-a-port
/// selection), this is a per-instance component, so multiple can coexist and each
/// owns its own channel ends. The motivating use is a mock HTTP server inside a
/// browser (a teaching sandbox) with no real listener; it is also the natural
/// deterministic test harness (no ports, no timing).
///
/// Boots through the fan-out exactly like [`HttpServer`]: a [`StartRunning<Boot>`]
/// whose `--server` selects `"channel"` starts the serve loop, which parks on the
/// host's [`Running<Response>`] keep-alive and tears down on its removal.
///
/// Runtime-only: it holds [`async_channel`] ends, which are not [`Reflect`], so
/// (unlike [`HttpServer`]) it is not markup-spawnable. Construct it with
/// [`ChannelHttpServer::new`].
#[derive(Component)]
#[component(on_add = on_add)]
#[require(ExchangeStats, ContinueRun<Boot, Response>)]
pub struct ChannelHttpServer {
	/// Inbound requests to dispatch.
	requests: Receiver<Request>,
	/// Outbound responses, one per dispatched request.
	responses: Sender<Response>,
}

/// The user-held end of a [`ChannelHttpServer`]: the "arbitrary user-defined
/// mechanism" that drives the mock server. Send a [`Request`], receive the matching
/// [`Response`].
pub struct ChannelHttpClient {
	/// Requests sent to the server.
	requests: Sender<Request>,
	/// Responses received from the server, one per request.
	responses: Receiver<Response>,
}

impl ChannelHttpServer {
	/// Creates a paired server and client over fresh channels.
	pub fn new() -> (ChannelHttpServer, ChannelHttpClient) {
		let (req_tx, req_rx) = async_channel::unbounded::<Request>();
		let (res_tx, res_rx) = async_channel::unbounded::<Response>();
		(
			ChannelHttpServer {
				requests: req_rx,
				responses: res_tx,
			},
			ChannelHttpClient {
				requests: req_tx,
				responses: res_rx,
			},
		)
	}
}

impl ChannelHttpClient {
	/// Send a request to the server, to be paired with a later [`Self::recv`] /
	/// [`Self::try_recv`].
	pub async fn send(&self, request: impl Into<Request>) -> Result<()> {
		self.requests
			.send(request.into())
			.await
			.map_err(|_| bevyhow!("channel http server closed"))
	}

	/// Await the next response from the server.
	pub async fn recv(&self) -> Result<Response> {
		self.responses
			.recv()
			.await
			.map_err(|_| bevyhow!("channel http server closed"))
	}

	/// Take a ready response without awaiting, if one has landed.
	pub fn try_recv(&self) -> Option<Response> { self.responses.try_recv().ok() }

	/// Send a request and await its response: [`Self::send`] then [`Self::recv`],
	/// for when the server is driven elsewhere (eg a background thread or a test
	/// drive loop).
	pub async fn request(
		&self,
		request: impl Into<Request>,
	) -> Result<Response> {
		self.send(request).await?;
		self.recv().await
	}
}

/// Shutdown signal for a running [`ChannelHttpServer`], mirroring the socket/http
/// servers: stored on the host, the receiver handed to the serve loop and signaled
/// when the host's [`Running<Response>`] is removed.
#[derive(Component)]
struct ChannelHttpServerShutdown(Option<OnceValue<()>>);

/// Registers the boot ([`StartRunning<Boot>`]) and teardown
/// (`On<Remove, Running<Response>>`) observers, mirroring [`HttpServer`].
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_action_in)
		.observe_any(on_running_removed);
}

/// Boots the serve loop on the boot fan-out, if `--server` selects `"channel"`.
/// Never resolves the boot call, so the host's [`Running<Response>`] parks it.
fn on_action_in(ev: On<StartRunning<Boot>>, mut commands: Commands) -> Result {
	let entity = ev.entity;
	if !ev.with(|boot| request_selects_server(boot, "channel"))? {
		return Ok(());
	}
	let (signal, shutdown) = oneshot::<()>();
	commands
		.entity(entity)
		.insert(ChannelHttpServerShutdown(Some(signal)))
		.queue_async_local(move |entity| {
			start_channel_http_server(entity, shutdown)
		});
	Ok(())
}

/// Tears down the serve loop when the host's [`Running<Response>`] is removed:
/// signals the shutdown channel. Idempotent: a missing handle is a no-op.
fn on_running_removed(
	ev: On<Remove, Running<Response>>,
	mut shutdowns: Query<&mut ChannelHttpServerShutdown>,
) {
	if let Ok(mut shutdown) = shutdowns.get_mut(ev.event().event_target())
		&& let Some(signal) = shutdown.0.take()
	{
		signal.signal(());
	}
}

/// The serve loop: drain requests off the channel, dispatch each through the host's
/// `Action<Request, Response>` slot via `entity.exchange`, and write the response
/// back. Parks like [`HttpServer`] (never resolves the boot call); ends when the
/// shutdown signal resolves or the request channel closes.
async fn start_channel_http_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	if !entity.is_alive().await {
		return Ok(());
	}
	// clone the channel ends out of the component; both are cheap handles.
	let (requests, responses) = entity
		.get::<ChannelHttpServer, _>(|server| {
			(server.requests.clone(), server.responses.clone())
		})
		.await?;

	let serve = {
		let entity = entity.clone();
		async move {
			while let Ok(request) = requests.recv().await {
				let response = entity.exchange(request).await;
				responses.send(response).await.ok();
			}
			Result::Ok(())
		}
	};
	// race the serve loop against teardown, mirroring `start_mini_http_server`.
	beet_core::exports::futures_lite::future::or(serve, async move {
		shutdown.wait().await;
		Result::Ok(())
	})
	.await
}

#[cfg(test)]
mod test {
	use super::*;

	/// Serve a real request/response over the channel transport: spawn a
	/// `ChannelHttpServer` with a mirror handler, boot it through the fan-out, then
	/// drive the app until the client's request round-trips. Drives to the bounded
	/// response condition (via [`AsyncRunner::poll_and_update`]) rather than settling
	/// a parked server, so it runs on native and wasm alike.
	#[beet_core::test]
	async fn serves_over_channel() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let (server, client) = ChannelHttpServer::new();
		let entity = app
			.world_mut()
			.spawn((
				server,
				exchange_handler(|cx| Response::ok().with_body(cx.take().body)),
			))
			.id();
		// boot through the fan-out (fire-and-forget: the call fans out and parks)
		app.world_mut().entity_mut(entity).run_async_local(
			|host| async move {
				host.call::<Boot, Response>(Boot::from(Request::get("/")))
					.await?;
				Ok(())
			},
		);
		// drive the app until the posted request round-trips to a response
		let response = AsyncRunner::poll_and_update(
			|| {
				app.update();
			},
			client.request(Request::post("/echo").with_body("hello")),
		)
		.await
		.unwrap();
		response.text().await.unwrap().xpect_eq("hello");
	}
}
