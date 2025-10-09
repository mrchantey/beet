use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use futures::ready;
use http_body_util::Full;
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
pub(super) fn run_server(
	settings: Res<ServerSettings>,
	mut async_commands: AsyncCommands,
) {
	let addr: SocketAddr = ([127, 0, 0, 1], settings.port).into();
	let handler = settings.handler();

	async_commands.run(async move |world| -> Result {
		let handler = handler.clone();
		let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
			.map_err(|e| bevyhow!("Failed to bind to {}: {}", addr, e))?;

		bevy::log::info!("Server listening on http://{}", addr);

		loop {
			let (tcp, addr) = listener
				.accept()
				.await
				.map_err(|e| bevyhow!("Failed to accept connection: {}", e))
				.unwrap();
			bevy::log::info!("New connection from: {}", addr);
			let io = BevyIo::new(tcp);

			let handler = handler.clone();
			let _entity_fut = world.run_async(async move |world| {
				let service = service_fn(move |req| {
					let world = world.clone();
					let handler = handler.clone();
					async move {
						let req = hyper_to_request(req).await;
						bevy::log::info!(
							"Request: {} {}",
							req.method(),
							req.parts.uri.path()
						);
						let res = handler(world.clone(), req).await;
						let res = response_to_hyper(res).await;
						bevy::log::info!("Response: {:?}", res.status());

						// non-await
						world.with_resource::<ServerStatus>(|mut status| {
							status.increment_requests();
						});
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
						bevy::log::trace!(
							"Connection closed due to header timeout (normal behavior)"
						);
					} else {
						bevy::log::error!(
							"Error serving connection: {:?}",
							err
						);
					}
				}
			});

			// Spawn each connection handler as a separate async task
			// bevy::tasks::IoTaskPool::get()
			// 	.spawn()
			// 	.detach();
		}
	});
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

	Request::from_parts(parts, body)
}

async fn response_to_hyper(res: Response) -> hyper::Response<Full<Bytes>> {
	match res.into_http().await {
		Ok(http_response) => {
			let (parts, body) = http_response.into_parts();
			hyper::Response::from_parts(parts, Full::new(body))
		}
		Err(_) => {
			error!("Failed to convert Response to hyper");
			let error_response = hyper::Response::builder()
				.status(500)
				.body(Full::new(Bytes::from("Internal Server Error")))
				.unwrap();
			error_response
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
