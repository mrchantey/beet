use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use futures::ready;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
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


/// System that starts the HTTP server using bevy's async commands
pub(super) fn run_server(
	settings: Res<ServerSettings>,
	mut async_commands: AsyncCommands,
) {
	// let addr
	let addr: SocketAddr = ([127, 0, 0, 1], settings.port).into();

	async_commands.run(async move |world| -> Result {
		bevy::log::info!("Starting Bevy HTTP server application...");


		// Bind to the port and listen for incoming TCP connections
		let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
			.map_err(|e| bevyhow!("Failed to bind to {}: {}", addr, e))?;

		bevy::log::info!("Bevy HTTP server listening on http://{}", addr);

		loop {
			// Accept incoming connections
			let (tcp, addr) = listener
				.accept()
				.await
				.map_err(|e| bevyhow!("Failed to accept connection: {}", e))
				.unwrap();
			bevy::log::info!("New connection from: {}", addr);
			let io = BevyIo::new(tcp);

			let _entity_fut = world.run_async(async |world| {
				let service = service_fn(move |req| {
					let world = world.clone();					
					handle_request(world, req)
				});

				if let Err(err) = http1::Builder::new()
					.timer(BevyTimer)
					.header_read_timeout(Duration::from_secs(2))
					// .keep_alive(false)
					.serve_connection(io, service)
					.await
				{
					if err.is_timeout()
						&& err.to_string()
							== ("read header from client timeout")
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

/// HTTP request handler that uses bevy's async world to manage state
async fn handle_request(
	world: AsyncWorld,
	req: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
	bevy::log::info!("Request: {} {}", req.method(), req.uri().path());
	// Increment request counter using async world
	let count = world
		.with_resource_then::<ServerStatus, _>(|mut status| {
			status.increment_requests().num_requests()
		})
		.await;

	let response_text = format!("Hello from Bevy! Request #{}", count);
	Ok(Response::new(Full::new(Bytes::from(response_text))))
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
		// Convert ReadBufCursor to &mut [u8] for futures::AsyncRead
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

// Timer implementation using bevy's async executor
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
