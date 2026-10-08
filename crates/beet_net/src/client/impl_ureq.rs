use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use std::io::Read;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::resolver::ResolvedSocketAddrs;
use ureq::unversioned::resolver::Resolver;
use ureq::unversioned::transport::DefaultConnector;
use ureq::unversioned::transport::NextTimeout;

pub(super) async fn send_ureq(req: Request) -> Result<Response> {
	super::send::check_https_features(&req)?;

	// read before the parts are taken: the policy rides the request, not its
	// parts, since a server never follows anything
	let max_redirects = req.redirects.max();
	let (parts, body) = req.into_parts();

	// 4xx/5xx are answers, not io failures: the caller wants the response
	// whatever its status, so only a connection error fails the request.
	let config = ureq::config::Config::builder()
		.http_status_as_error(false)
		// `0` answers the `3xx` to the caller rather than erroring:
		// `max_redirects_do_error` is `max_redirects > 0 && ..`
		.max_redirects(max_redirects)
		// the socket and the tls handshake, never the response: a stalled
		// connect otherwise waits out the operating system's SYN retries
		.timeout_connect(Some(CONNECT_TIMEOUT))
		.xmap(with_tls)
		.build();
	let agent = ureq::Agent::with_parts(
		config,
		DefaultConnector::new(),
		ReachableFirst::default(),
	);

	// Convert to http::Request
	let http_parts: http::request::Parts = parts.try_into()?;
	let body = body.into_bytes().await?.to_vec();
	let http_req = http::Request::from_parts(http_parts, body);

	// Run the whole blocking exchange on a thread pool, the body read included:
	// `agent.run` resolves only the status and headers, handing back a reader that
	// pulls the body straight off the socket. Reading it on the caller's thread
	// blocks it, and without `bevy_multithreaded` that thread is the world thread
	// every system, every connection and every other task also runs on (see
	// [`AsyncSpawner`]). The sharp case is a beet process fetching its own server
	// over the loopback port: the blocked thread is the one that owes it the body,
	// so neither side can finish. Only [`create_streaming_body`] may cross back,
	// and it does its reads on the same pool.
	blocking::unblock(move || {
		agent
			.run(http_req)
			.map_err(BevyError::from)
			.and_then(into_response)
	})
	.await
}

/// The longest a request may take to connect, tls included.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// ureq's [`DefaultResolver`] with the reachable address first.
///
/// ureq tries resolved addresses one at a time in the order the system gives
/// them, and a network that advertises an IPv6 route it cannot carry puts every
/// IPv6 address first: each costs its share of the connect timeout (without
/// one, the operating system's SYN retries, about two minutes) before an IPv4
/// address is tried. So the families are interleaved and raced as RFC 8305
/// (Happy Eyeballs) does, one attempt started every
/// [`ATTEMPT_DELAY`](Self::ATTEMPT_DELAY), and the first address to connect
/// leads the list. The probe connection is dropped and ureq opens its own to
/// the winner, one extra handshake per request.
#[derive(Debug, Default)]
struct ReachableFirst(DefaultResolver);

impl ReachableFirst {
	/// The stagger between attempts, RFC 8305's recommended default.
	const ATTEMPT_DELAY: Duration = Duration::from_millis(250);
	/// How long one attempt may take before the race leaves it behind.
	const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);

	/// `addrs` with the families alternating, the system's preferred family
	/// first and each family's own order kept.
	fn interleave(addrs: &[SocketAddr]) -> Vec<SocketAddr> {
		let Some(first) = addrs.first() else {
			return Vec::new();
		};
		let (preferred, other): (Vec<_>, Vec<_>) = addrs
			.iter()
			.copied()
			.partition(|addr| addr.is_ipv6() == first.is_ipv6());
		let mut other = other.into_iter();
		let mut interleaved = Vec::with_capacity(addrs.len());
		for addr in preferred {
			interleaved.push(addr);
			interleaved.extend(other.next());
		}
		interleaved.extend(other);
		interleaved
	}

	/// The first of `addrs` to accept a TCP connection, each attempt started
	/// [`ATTEMPT_DELAY`](Self::ATTEMPT_DELAY) after the last; [`None`] when
	/// none connects. An attempt still running when another wins finishes on
	/// its own thread and is dropped.
	fn race(addrs: &[SocketAddr]) -> Option<SocketAddr> {
		let (sender, receiver) = mpsc::channel();
		let won = Arc::new(AtomicBool::new(false));
		for (index, addr) in addrs.iter().copied().enumerate() {
			let (sender, won) = (sender.clone(), won.clone());
			std::thread::spawn(move || {
				std::thread::sleep(Self::ATTEMPT_DELAY * index as u32);
				if !won.load(Ordering::Relaxed)
					&& TcpStream::connect_timeout(&addr, Self::ATTEMPT_TIMEOUT)
						.is_ok()
				{
					sender.send(addr).ok();
				}
			});
		}
		// every attempt holds a sender, so a race nobody wins disconnects
		drop(sender);
		let winner = receiver
			.recv_timeout(
				Self::ATTEMPT_TIMEOUT
					+ Self::ATTEMPT_DELAY * addrs.len() as u32,
			)
			.ok();
		won.store(true, Ordering::Relaxed);
		winner
	}
}

impl Resolver for ReachableFirst {
	fn resolve(
		&self,
		uri: &ureq::http::Uri,
		config: &ureq::config::Config,
		timeout: NextTimeout,
	) -> Result<ResolvedSocketAddrs, ureq::Error> {
		let mut resolved = self.0.resolve(uri, config, timeout)?;
		if resolved.len() > 1 {
			let mut ordered = Self::interleave(&resolved);
			// nobody connecting leaves the interleaved order, and ureq's own
			// attempt reports the failure
			if let Some(winner) = Self::race(&ordered) {
				ordered.retain(|addr| *addr != winner);
				ordered.insert(0, winner);
			}
			resolved.copy_from_slice(&ordered);
		}
		Ok(resolved)
	}
}

/// The `ureq` agent-config builder, named so the tls branches below can hand
/// one through without restating the typestate.
type AgentConfig = ureq::config::ConfigBuilder<ureq::typestate::AgentScope>;

// The tls provider is the only part of the agent config the feature set
// changes: `native-tls` wins where both are on, and a build with neither
// never names `ureq::tls`, which is itself gated behind them.
cfg_if! {
	if #[cfg(feature = "native-tls")] {
		fn with_tls(config: AgentConfig) -> AgentConfig {
			config.tls_config(
				ureq::tls::TlsConfig::builder()
					.provider(ureq::tls::TlsProvider::NativeTls)
					.build(),
			)
		}
	} else if #[cfg(feature = "rustls-tls")] {
		fn with_tls(config: AgentConfig) -> AgentConfig {
			config.tls_config(
				ureq::tls::TlsConfig::builder()
					.provider(ureq::tls::TlsProvider::Rustls)
					.build(),
			)
		}
	} else {
		fn with_tls(config: AgentConfig) -> AgentConfig { config }
	}
}

fn into_response(res: http::Response<ureq::Body>) -> Result<Response> {
	// Check if this is a streaming response (SSE or chunked)
	let is_event_stream = res
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.map_or(false, |ct| ct.contains("text/event-stream"));

	let is_chunked = res
		.headers()
		.get("transfer-encoding")
		.and_then(|v| v.to_str().ok())
		.map_or(false, |te| te.contains("chunked"));

	let should_stream = is_event_stream || is_chunked;

	// Build ResponseParts with headers
	let mut parts = ResponseParts::new(res.status().into());
	for (key, value) in res.headers().iter() {
		if let Ok(value_str) = value.to_str() {
			parts.headers.set_raw(key.to_string(), value_str);
		}
	}

	let body = if should_stream {
		// Create a streaming body for SSE/chunked responses
		create_streaming_body(res.into_body())
	} else {
		// `Body::read_to_vec` caps at 10MB; bypass via the unlimited reader.
		let mut bytes_vec = Vec::new();
		res.into_body().into_reader().read_to_end(&mut bytes_vec)?;
		Body::Bytes(Bytes::from(bytes_vec))
	};

	Ok(Response::from_parts(parts, Bytes::new()).with_body(body))
}

/// Creates a streaming body from a ureq body reader.
/// Spawns blocking reads on a thread pool and sends chunks through a channel.
fn create_streaming_body(ureq_body: ureq::Body) -> Body {
	use futures::stream;

	let (sender, receiver) = async_channel::bounded::<Result<Bytes>>(16);

	// Spawn the blocking reader on a thread pool
	blocking::unblock(move || {
		let mut reader = ureq_body.into_reader();
		let mut buf = vec![0u8; 8192];

		loop {
			match reader.read(&mut buf) {
				Ok(0) => {
					// EOF reached
					break;
				}
				Ok(n) => {
					let chunk = Bytes::copy_from_slice(&buf[..n]);
					// If receiver is dropped, stop reading
					if sender.send_blocking(Ok(chunk)).is_err() {
						break;
					}
				}
				Err(err) => {
					let _ = sender.send_blocking(Err(BevyError::from(err)));
					break;
				}
			}
		}
	})
	.detach();

	// Convert the receiver into a stream
	let byte_stream = stream::unfold(receiver, |rx| async move {
		match rx.recv().await {
			Ok(result) => Some((result, rx)),
			Err(_) => None, // Channel closed
		}
	});

	Body::stream(byte_stream)
}

#[cfg(all(test, feature = "server", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A body far larger than any loopback socket buffer, so the server cannot
	/// hand the whole response off to the kernel and walk away: it has to still be
	/// writing while the client is reading.
	const BODY_LEN: usize = 8 * 1024 * 1024;
	/// Long enough that only a genuine wedge trips it.
	const BUDGET: Duration = Duration::from_secs(20);

	/// Reading a response body must not block the caller's thread.
	///
	/// Regression test for a self-deadlock: `into_response` used to run outside
	/// [`blocking::unblock`], so the body read happened on whichever thread polled
	/// [`send_ureq`]. In a build without `bevy_multithreaded` (the deployed
	/// default) that is the world thread, which is also the thread serving the
	/// connection — so a beet process fetching its own http server, exactly what
	/// the TUI image path does over the loopback port, blocked the writer from
	/// inside the reader and neither side could finish. Pre-fix this hangs until
	/// the budget; post-fix the body arrives whole.
	#[beet_core::test(timeout_ms = 40_000)]
	async fn body_read_does_not_block_the_caller() {
		let (server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let url = server.local_url();
		let (send, recv) = OnceValue::<Result<usize>>::oneshot();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			app.world_mut().spawn((server, on_spawn, children![
				exchange_ext::handler(|_| {
					Response::ok().with_body(vec![b'x'; BODY_LEN])
				})
			]));
			// fetch this very server from its own world thread, the shape the TUI
			// image path takes over the loopback port.
			app.world_mut().run_async_local(move |_| async move {
				send.signal(
					async move {
						Request::get(&url).send().await?.body.into_bytes().await
					}
					.await
					.map(|bytes| bytes.len()),
				);
			});
			app.run();
		});

		async_ext::timeout(BUDGET, recv.wait())
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(BODY_LEN);
	}

	/// [`Redirects::None`] answers the `3xx` itself, so a caller can carry a
	/// session across a hop the client would otherwise strip it from.
	#[beet_core::test]
	async fn does_not_follow_when_told_not_to() {
		let (server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let url = server.local_url();
		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((server, on_spawn, children![exchange_ext::handler(
					|cx| match cx.take().url().path_string().as_str() {
						"/landed" => Response::ok_text("landed"),
						_ => Response::temporary_redirect("/landed"),
					}
				)]))
				.run();
		});
		time_ext::sleep_millis(100).await;

		// followed by default, the way every other caller expects
		Request::get(&url)
			.send()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq("landed");

		let response = Request::get(&url)
			.with_redirects(Redirects::None)
			.send()
			.await
			.unwrap();
		response.status().as_u16().xpect_eq(307);
		response
			.headers()
			.first_raw("location")
			.xpect_eq(Some("/landed"));
	}
}

#[cfg(test)]
mod reachable_first_test {
	use super::ReachableFirst;
	use beet_core::prelude::*;
	use std::net::SocketAddr;
	use std::net::TcpListener;

	fn addr(text: &str) -> SocketAddr { text.parse().unwrap() }

	/// The families alternate from the system's first choice, each keeping its
	/// own order, so one unreachable family cannot hold every early attempt.
	#[beet_core::test]
	fn interleaves_the_families() {
		ReachableFirst::interleave(&[
			addr("[2606:4700::1]:443"),
			addr("[2606:4700::2]:443"),
			addr("[2606:4700::3]:443"),
			addr("104.19.192.1:443"),
			addr("104.19.192.2:443"),
		])
		.xpect_eq(vec![
			addr("[2606:4700::1]:443"),
			addr("104.19.192.1:443"),
			addr("[2606:4700::2]:443"),
			addr("104.19.192.2:443"),
			addr("[2606:4700::3]:443"),
		]);
	}

	/// The address that accepts wins inside a few staggers, however long the
	/// one ahead of it would have stalled.
	///
	/// REGRESSION: a network advertising an IPv6 route it could not carry hung
	/// every Cloudflare call for two minutes per IPv6 address, since ureq tried
	/// them one at a time with no connect timeout. The documentation address
	/// stands in for the dead route: dropped where there is a route, refused
	/// where there is none, the race goes on either way.
	#[beet_core::test]
	fn races_to_the_reachable_address() {
		let listener = TcpListener::bind("127.0.0.1:0").unwrap();
		let reachable = listener.local_addr().unwrap();
		let start = Instant::now();
		ReachableFirst::race(&[addr("192.0.2.1:443"), reachable])
			.xpect_eq(Some(reachable));
		start.elapsed().xpect_less_than(Duration::from_secs(2));
	}
}
