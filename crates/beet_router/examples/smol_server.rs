use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

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

// Wrapper to make async-io's TcpStream work with hyper's IO traits
struct SmolIo<S> {
	inner: S,
}

impl<S> SmolIo<S> {
	fn new(stream: S) -> Self { Self { inner: stream } }
}

impl<S> hyper::rt::Read for SmolIo<S>
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

impl<S> hyper::rt::Write for SmolIo<S>
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

// Timer implementation for async-io
#[derive(Clone, Debug)]
struct SmolTimer;

impl Timer for SmolTimer {
	fn sleep(&self, duration: Duration) -> Pin<Box<dyn Sleep>> {
		Box::pin(SmolSleep {
			inner: async_io::Timer::after(duration),
		})
	}

	fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn Sleep>> {
		Box::pin(SmolSleep {
			inner: async_io::Timer::at(deadline),
		})
	}

	fn reset(&self, sleep: &mut Pin<Box<dyn Sleep>>, new_deadline: Instant) {
		if let Some(sleep) = sleep.as_mut().downcast_mut_pin::<SmolSleep>() {
			sleep.reset(new_deadline)
		}
	}
}

#[pin_project]
struct SmolSleep {
	#[pin]
	inner: async_io::Timer,
}

impl Future for SmolSleep {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self.project().inner.poll(cx) {
			Poll::Ready(_) => Poll::Ready(()),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl Sleep for SmolSleep {}

impl SmolSleep {
	fn reset(self: Pin<&mut Self>, deadline: Instant) {
		self.project().inner.as_mut().set_at(deadline);
	}
}

// Async function that handles requests
async fn hello(
	_: Request<impl hyper::body::Body>,
) -> Result<Response<Full<Bytes>>, Infallible> {
	Ok(Response::new(Full::new(Bytes::from("Hello dssdpoop!"))))
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	// Use async-io's block_on for simplicity
	async_io::block_on(async {
		// This address is localhost
		let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

		// Bind to the port and listen for incoming TCP connections
		let listener = async_io::Async::<std::net::TcpListener>::bind(addr)?;
		println!("Listening on http://{}", addr);

		let executor = std::sync::Arc::new(async_executor::Executor::new());

		// Spawn executor runner in background
		let executor_runner = executor.clone();
		let _executor_handle = std::thread::spawn(move || {
			async_io::block_on(
				executor_runner.run(std::future::pending::<()>()),
			)
		});

		loop {
			// Accept incoming connections
			let (tcp, _) = listener.accept().await?;
			let io = SmolIo::new(tcp);

			executor
				.spawn(async move {
					if let Err(err) = http1::Builder::new()
						.timer(SmolTimer)
						.serve_connection(io, service_fn(hello))
						.await
					{
						println!("Error serving connection: {:?}", err);
					}
				})
				.detach();
		}
	})
}
