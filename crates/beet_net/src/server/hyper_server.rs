use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use futures::ready;
use http_body_util::BodyExt;
use http_body_util::Full;
use http_body_util::StreamBody;
use hyper::body::Frame;
use hyper::rt::Sleep;
use hyper::rt::Timer;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use pin_project::pin_project;
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// A hyper/bevy server.
///
/// This async function contains unopinionated machinery for handling
/// hyper requests.
/// See [`HttpServer`] for customizing handlers.
pub async fn start_hyper_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	let addr = entity
		.get::<HttpServer, SocketAddr>(|server| server.socket_addr())
		.await?;

	let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
		.map_err(|err| bevyhow!("Failed to bind to {}: {}", addr, err))?;

	start_hyper_server_with_tcp(entity, listener, shutdown).await
}

/// Like [`start_hyper_server`] but accepts a pre-bound TCP listener,
/// eliminating port race conditions in tests.
pub async fn start_hyper_server_with_tcp(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
	shutdown: OnceValueRx<()>,
) -> Result {
	let addr = listener
		.get_ref()
		.local_addr()
		.map_err(|err| bevyhow!("Failed to get local address: {}", err))?;
	info!("Server listening on http://{}", addr);
	// register the resolved port as the process loopback port when canonical (the
	// mini server does the same), so an authority-less request loops back here. An
	// entity with no `HttpServer` still claims it, matching the `canonical` default.
	if entity
		.get::<HttpServer, bool>(|server| server.canonical)
		.await
		.unwrap_or(true)
	{
		HttpServer::set_current_port(addr.port());
	}

	// race the accept loop against the shutdown signal: signalling drops the loop
	// future, releasing the listener so the port closes (the mini server pattern).
	beet_core::exports::futures_lite::future::or(
		hyper_accept_loop(entity, listener),
		async move {
			shutdown.wait().await;
			Result::Ok(())
		},
	)
	.await
}

/// The hyper accept loop: serve each connection on its own spawned task. Diverges
/// (only the shutdown race in [`start_hyper_server_with_tcp`] ends it).
async fn hyper_accept_loop(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
) -> Result {
	loop {
		let (tcp, addr) = listener
			.accept()
			.await
			.map_err(|err| bevyhow!("Failed to accept connection: {}", err))
			.unwrap();
		trace!("New connection from: {}", addr);
		let io = BevyIo::new(tcp);

		entity
			.run_async_local(async move |entity| {
				// pass an AsyncEntity to the service_fn
				let service = service_fn(move |mut req| {
					let entity = entity.clone();

					async move {
						// grab the upgrade future before consuming the request; if
						// the route answers with a `101` we drive it to a `Socket`
						#[cfg(all(
							feature = "tungstenite",
							not(target_arch = "wasm32")
						))]
						let on_upgrade = hyper::upgrade::on(&mut req);
						let req = hyper_to_request(req).await;
						let res = entity.exchange(req).await;
						#[cfg(all(
							feature = "tungstenite",
							not(target_arch = "wasm32")
						))]
						if http_ext::is_websocket_response(&res) {
							spawn_hyper_upgrade(entity, on_upgrade).await;
						}
						let res = response_to_hyper(res).await;
						res.xok::<Infallible>()
					}
				});

				// `.with_upgrades()`: keep the connection alive past the `101` so
				// hyper can yield the upgraded IO to `hyper::upgrade::on`.
				if let Err(err) = http1::Builder::new()
					.timer(BevyTimer)
					.header_read_timeout(Duration::from_secs(2))
					// .keep_alive(false)
					.serve_connection(io, service)
					.with_upgrades()
					.await
				{
					if err.is_timeout()
						&& err.xfmt_debug() == "hyper::Error(HeaderTimeout)"
					{
						trace!(
							"Connection closed due to header timeout (normal behavior)"
						);
					} else {
						error!("Error serving connection: {:?}", err);
					}
				}
			})
			.await
			.ok();
	}
}

/// Drive a hyper connection upgrade to a [`Socket`]: await the upgraded IO,
/// adapt it to futures IO ([`HyperIo`]), wrap it (`Role::Server`, no
/// re-handshake), spawn it as a [`Socket`] entity, and fire
/// [`OnWebSocketUpgrade`] for the socket layer to adopt, mirroring the
/// `mini_http_server` hand-off. Runs `_local` for the thread-bound reader.
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
async fn spawn_hyper_upgrade(
	entity: AsyncEntity,
	on_upgrade: hyper::upgrade::OnUpgrade,
) {
	// awaiting only spawns the detached task (which then awaits `on_upgrade` in
	// the background); it does not block the `101` response.
	entity
		.world()
		.clone()
		.run_async_local(async move |world| -> Result {
			let upgraded = on_upgrade
				.await
				.map_err(|err| bevyhow!("hyper upgrade failed: {err}"))?;
			let socket =
				crate::sockets::socket_from_upgraded(HyperIo::new(upgraded))
					.await;
			world
				.with(move |world: &mut World| {
					let socket = world.spawn(socket).id();
					world
						.trigger(crate::sockets::OnWebSocketUpgrade { socket });
				})
				.await;
			Ok(())
		})
		.await;
}

async fn hyper_to_request(
	req: hyper::Request<hyper::body::Incoming>,
) -> Request {
	let (parts, body) = req.into_parts();

	// Convert hyper body into a stream
	let stream = http_body_util::BodyStream::new(body);
	let stream = stream.map(|result| match result {
		Ok(frame) => match frame.into_data() {
			Ok(data) => Ok(data),
			Err(_) => Err(bevyhow!("Failed to convert frame to data")),
		},
		Err(err) => Err(bevyhow!("Body stream error: {:?}", err)),
	});

	// Create body based on size
	let body = Body::stream(stream);

	Request::from_parts(RequestParts::from(parts), body)
}

async fn response_to_hyper(
	res: Response,
) -> hyper::Response<http_body_util::combinators::BoxBody<Bytes, std::io::Error>>
{
	let (parts, body) = res.into_parts();

	// Convert our ResponseParts to http::response::Parts
	let http_parts: http::response::Parts =
		parts.try_into().unwrap_or_else(|err| {
			error!("Failed to convert response parts: {}", err);
			http::Response::builder()
				.status(http::StatusCode::INTERNAL_SERVER_ERROR)
				.body(())
				.unwrap()
				.into_parts()
				.0
		});

	match body {
		Body::Bytes(bytes) => {
			let body = Full::new(bytes).map_err(|never| match never {}).boxed();
			hyper::Response::from_parts(http_parts, body)
		}
		Body::Stream(stream) => {
			// Convert our stream to a stream of Frames
			let frame_stream = stream.map(|result| {
				result.map(Frame::data).map_err(|e| {
					std::io::Error::new(
						std::io::ErrorKind::Other,
						e.to_string(),
					)
				})
			});

			let body = BodyExt::boxed(StreamBody::new(frame_stream));
			hyper::Response::from_parts(http_parts, body)
		}
	}
}

// Wrapper to make async-io's TcpStream work with hyper's IO traits
struct BevyIo<S> {
	inner: S,
}

impl<S> BevyIo<S> {
	fn new(stream: S) -> Self { Self { inner: stream } }
}

impl<S> hyper::rt::Read for BevyIo<S>
where
	S: futures::AsyncRead + Unpin,
{
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		mut buf: hyper::rt::ReadBufCursor<'_>,
	) -> Poll<io::Result<()>> {
		let slice = unsafe {
			std::slice::from_raw_parts_mut(
				buf.as_mut().as_mut_ptr() as *mut u8,
				buf.as_mut().len(),
			)
		};

		let n = ready!(Pin::new(&mut self.inner).poll_read(cx, slice))?;
		unsafe { buf.advance(n) };
		Poll::Ready(Ok(()))
	}
}

impl<S> hyper::rt::Write for BevyIo<S>
where
	S: futures::AsyncWrite + Unpin,
{
	fn poll_write(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<Result<usize, io::Error>> {
		Pin::new(&mut self.inner).poll_write(cx, buf)
	}

	fn poll_flush(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), io::Error>> {
		Pin::new(&mut self.inner).poll_flush(cx)
	}

	fn poll_shutdown(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), io::Error>> {
		Pin::new(&mut self.inner).poll_close(cx)
	}
}

/// The inverse of [`BevyIo`]: adapts a `hyper::rt::Read + hyper::rt::Write` (eg
/// the [`hyper::upgrade::Upgraded`] IO) to the futures IO traits
/// `async-tungstenite` needs, so an upgraded hyper connection becomes a
/// [`Socket`](crate::sockets::Socket).
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
struct HyperIo<S> {
	inner: S,
}

#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
impl<S> HyperIo<S> {
	fn new(stream: S) -> Self { Self { inner: stream } }
}

#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
impl<S> futures::AsyncRead for HyperIo<S>
where
	S: hyper::rt::Read + Unpin,
{
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut [u8],
	) -> Poll<io::Result<usize>> {
		let mut read_buf = hyper::rt::ReadBuf::new(buf);
		ready!(Pin::new(&mut self.inner).poll_read(cx, read_buf.unfilled()))?;
		Poll::Ready(Ok(read_buf.filled().len()))
	}
}

#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
impl<S> futures::AsyncWrite for HyperIo<S>
where
	S: hyper::rt::Write + Unpin,
{
	fn poll_write(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<io::Result<usize>> {
		Pin::new(&mut self.inner).poll_write(cx, buf)
	}

	fn poll_flush(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<io::Result<()>> {
		Pin::new(&mut self.inner).poll_flush(cx)
	}

	fn poll_close(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<io::Result<()>> {
		Pin::new(&mut self.inner).poll_shutdown(cx)
	}
}

#[derive(Clone, Debug)]
struct BevyTimer;

impl Timer for BevyTimer {
	fn sleep(&self, duration: Duration) -> Pin<Box<dyn Sleep>> {
		Box::pin(BevySleep {
			inner: async_io::Timer::after(duration),
		})
	}

	fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn Sleep>> {
		Box::pin(BevySleep {
			inner: async_io::Timer::at(deadline),
		})
	}

	fn reset(&self, sleep: &mut Pin<Box<dyn Sleep>>, new_deadline: Instant) {
		if let Some(sleep) = sleep.as_mut().downcast_mut_pin::<BevySleep>() {
			sleep.reset(new_deadline)
		}
	}
}

#[pin_project]
struct BevySleep {
	#[pin]
	inner: async_io::Timer,
}

impl Future for BevySleep {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self.project().inner.poll(cx) {
			Poll::Ready(_) => Poll::Ready(()),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl Sleep for BevySleep {}

impl BevySleep {
	fn reset(self: Pin<&mut Self>, deadline: Instant) {
		self.project().inner.as_mut().set_at(deadline);
	}
}

#[cfg(test)]
#[cfg(feature = "ureq")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bytes::Bytes;
	use std::time::Duration;
	use std::time::Instant;

	#[beet_core::test]
	async fn roundtrip() {
		super::super::http_server::test::test_server(
			start_hyper_server_with_tcp,
		)
		.await;
	}

	#[beet_core::test]
	async fn works() {
		let server = HttpServer::new_test(start_hyper_server_with_tcp);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((
					server,
					exchange_handler(move |req| {
						Response::ok().with_body(req.take().body)
					}),
				))
				.run();
		});
		time_ext::sleep_millis(100).await;
		for _ in 0..10 {
			Request::post(&url)
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.xpect_ok();
		}
	}
	#[beet_core::test]
	async fn stream_roundtrip() {
		let server = HttpServer::new_test(start_hyper_server_with_tcp);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((server, mirror_exchange()))
				.run();
		});
		time_ext::sleep_millis(100).await;
		Request::post(url)
			.with_body_stream(bevy::tasks::futures_lite::stream::iter(vec![
				Ok(Bytes::from("foo")),
				Ok(Bytes::from("bar")),
				Ok(Bytes::from("bazz")),
			]))
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq("foobarbazz");
	}

	// asserts stream behavior with timestamps and delays
	#[beet_core::test]
	async fn stream_timestamp() {
		let server = HttpServer::new_test(start_hyper_server_with_tcp);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((
					exchange_handler(move |req| {
						// Server adds 100ms delay per chunk
						let delayed_stream = futures::stream::unfold(
							req.take().body,
							|mut body| async move {
								match body.next().await {
									Ok(Some(chunk)) => {
										time_ext::sleep(Duration::from_millis(
											100,
										))
										.await;
										Some((Ok(chunk), body))
									}
									Ok(None) => None,
									Err(e) => Some((Err(e), body)),
								}
							},
						);
						Response::ok().with_body(Body::stream(delayed_stream))
					}),
					server,
				))
				.run();
		});
		time_ext::sleep_millis(100).await;

		let start_time = Instant::now();

		// Create timestamped stream that starts after request is sent
		let timestamped_stream =
			futures::stream::unfold(0usize, move |count| async move {
				if count >= 3 {
					return None;
				}

				// Wait 100ms between chunks (including initial delay)
				time_ext::sleep(Duration::from_millis(100)).await;

				let elapsed = start_time.elapsed().as_millis() as u64;
				let timestamp_data = format!("{}:{}\n", count, elapsed);

				Some((Ok(Bytes::from(timestamp_data)), count + 1))
			});

		let mut response_stream = Request::post(url)
			.with_body_stream(timestamped_stream)
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap()
			.body;

		// Collect all response data
		let mut all_data = Vec::new();
		while let Some(chunk) = response_stream.next().await.unwrap() {
			all_data.extend_from_slice(&chunk);
		}
		let response_str = String::from_utf8(all_data).unwrap();
		let final_elapsed = start_time.elapsed().as_millis() as u64;

		// Parse each line (chunk)
		let lines: Vec<&str> = response_str.trim().split('\n').collect();
		lines.len().xpect_eq(3);

		for (chunk_count, line) in lines.iter().enumerate() {
			// Parse the timestamp from the chunk
			let parts: Vec<&str> = line.split(':').collect();
			let chunk_index: usize = parts[0].parse().unwrap();

			chunk_index.xpect_eq(chunk_count);
		}

		// Verify total time is reasonable: ~300ms for 3 chunks with 100ms delays each
		// Use generous upper bound to account for system load variance
		final_elapsed.xpect_greater_or_equal_to(300);
		final_elapsed.xpect_less_or_equal_to(2000);
	}
}

#[cfg(test)]
#[cfg(feature = "tungstenite")]
mod upgrade_test {
	use crate::prelude::*;
	use crate::sockets::*;
	// explicit: `sockets::Message` (the enum) must win over bevy's `Message`
	// trait, both pulled in by the globs above
	use crate::sockets::Message;
	use beet_core::prelude::*;

	/// The hyper backend's same-port upgrade: a route returning
	/// [`WebSocketUpgrade`] drives `hyper::upgrade::on` to a [`Socket`] entity
	/// (via [`HyperIo`]), and the channel echoes over it. Mirrors the
	/// `mini_http_server` upgrade test.
	#[beet_core::test]
	async fn upgrades_to_socket() {
		let server = HttpServer::new_test(super::start_hyper_server_with_tcp);
		let port = server.0.port.unwrap();
		let landed = Store::<Vec<Entity>>::default();
		let captor = landed.clone();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			app.world_mut().spawn((
				server,
				exchange_handler(|cx| {
					WebSocketUpgrade::from_request(&cx).into()
				}),
			));
			app.world_mut()
				.add_observer(move |ev: On<OnWebSocketUpgrade>| {
					captor.push(ev.event().socket);
				});
			// global recv observer echoes text, installed before any reader fires
			app.world_mut().add_observer(
				|ev: On<MessageRecv>, mut commands: Commands| {
					if let Message::Text(text) = ev.event().inner() {
						commands.entity(ev.original_target()).trigger_target(
							MessageSend(Message::text(text.clone())),
						);
					}
				},
			);
			app.run();
		});
		time_ext::sleep_millis(200).await;

		let mut client = Socket::connect(format!("ws://127.0.0.1:{port}"))
			.await
			.unwrap();
		client.send(Message::text("over-hyper")).await.unwrap();

		let mut echoed = None;
		for _ in 0..40 {
			if let Some(Ok(Message::Text(text))) = client.next().await {
				echoed = Some(text);
				break;
			}
		}
		echoed.xpect_eq(Some("over-hyper".to_string()));
		landed.get().len().xpect_eq(1usize);
		client.close(None).await.ok();
	}
}
