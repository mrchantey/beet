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
use send_wrapper::SendWrapper;
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// A hyper/bevy server
/// This bevy system contains unopinionated machinery for handling
/// hyper requests.
/// See [`Server::handler`] for customizing handlers
pub(super) fn start_hyper_server(
	In(entity): In<Entity>,
	query: Query<&HttpServer>,
	mut async_commands: AsyncCommands,
) -> Result {
	let server = query.get(entity)?;
	let addr: SocketAddr = ([127, 0, 0, 1], server.port).into();

	async_commands.run(async move |world| -> Result {
		let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
			.map_err(|e| bevyhow!("Failed to bind to {}: {}", addr, e))?;

		info!("Server listening on http://{}", addr);

		loop {
			let (tcp, addr) = listener
				.accept()
				.await
				.map_err(|e| bevyhow!("Failed to accept connection: {}", e))
				.unwrap();
			trace!("New connection from: {}", addr);
			let io = BevyIo::new(tcp);

			let _entity_fut = world.run_async(async move |world| {
				// pass an AsyncWorld to the service_fn
				let service = service_fn(move |req| {
					let world = world.clone();

					async move {
						let req = hyper_to_request(req).await;
						let res = world.entity(entity).exchange(req).await;
						let res = response_to_hyper(res).await;
						res.xok::<Infallible>()
					}
				});

				if let Err(err) = http1::Builder::new()
					.timer(BevyTimer)
					.header_read_timeout(Duration::from_secs(2))
					// .keep_alive(false)
					.serve_connection(io, service)
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
			});
		}
	});
	Ok(())
}


async fn hyper_to_request(
	req: hyper::Request<hyper::body::Incoming>,
) -> Request {
	let (parts, body) = req.into_parts();

	// Convert hyper body into a stream
	let stream = http_body_util::BodyStream::new(body);
	let stream = Box::pin(stream.map(|result| match result {
		Ok(frame) => match frame.into_data() {
			Ok(data) => Ok(data),
			Err(_) => Err(bevyhow!("Failed to convert frame to data")),
		},
		Err(err) => Err(bevyhow!("Body stream error: {:?}", err)),
	}));

	// Create body based on size
	let body = Body::Stream(SendWrapper::new(stream));

	Request::from_parts(RequestParts::from(parts), body)
}

async fn response_to_hyper(
	res: Response,
) -> hyper::Response<http_body_util::combinators::BoxBody<Bytes, std::io::Error>>
{
	let (parts, body) = res.into_parts();

	// Convert our ResponseParts to http::response::Parts
	let http_parts: http::response::Parts =
		parts.try_into().unwrap_or_else(|_| {
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
			let frame_stream = stream.take().map(|result| {
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
	async fn works() {
		let server = HttpServer::new_test();

		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					server,
					handler_exchange(move |mut entity, req| {
						let count = entity.world_scope(|world: &mut World| {
							world.query_once::<&ExchangeStats>()[0]
								.request_count()
						});
						assert!(count < 99999);
						Response::ok().with_body(req.body)
					}),
				))
				.run();
		});
		time_ext::sleep_millis(50).await;
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
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((server, mirror_exchange()))
				.run();
		});
		time_ext::sleep_millis(50).await;
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
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					handler_exchange(move |_, req| {
						// Server adds 100ms delay per chunk
						let delayed_stream = futures::stream::unfold(
							req.body,
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
		time_ext::sleep_millis(50).await;

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
