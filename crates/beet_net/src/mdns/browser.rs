//! The agnostic mDNS service **Browser** engine: bytes-and-world, no sockets.
//!
//! This is the platform-independent half of the mDNS browser. It enumerates
//! instances of a service type on the LAN and surfaces each as **one entity**
//! carrying an [`MDnsService`] component, so downstream code observes discoveries
//! with a plain `Query<&MDnsService>` (or `On<Add, MDnsService>` /
//! `On<Remove, MDnsService>` observers) instead of a bespoke event/resource API:
//!
//! - a low-level [`UdpPacket`] `{ from, bytes }` global event is the seam: every
//!   inbound datagram becomes one, regardless of how it arrived (the std driver
//!   in this crate, or a downstream esp embassy loop bridging across silicon);
//! - an [`observer`](handle_udp_packet) parses each datagram's mDNS records
//!   ([`super::wire`]) and reconciles the live set of [`MDnsService`] entities:
//!   spawning one when an instance first resolves, updating it in place when its
//!   host/port/addr change, and despawning it on an mDNS goodbye (TTL=0).
//!
//! All of the above is `&mut World`-only, so it is fully agnostic. The only
//! platform-specific piece is *driving the socket*: binding it, joining
//! multicast, periodically sending the `PTR` query, and turning each
//! `recv_from` into a [`UdpPacket`]. On std the socket runtime and the world
//! runtime coincide, so [`run_mdns_browser`] drives the socket directly with no
//! bridge (see [`super::super::udp::impl_async_io`]). On esp that loop lives on
//! embassy and bridges datagrams into `world.trigger(UdpPacket)`; everything in
//! *this* module is reused unchanged.

use super::wire;
use super::wire::Record;
use beet_core::prelude::*;
use core::net::Ipv4Addr;
use core::net::SocketAddr;

/// A single inbound UDP datagram, surfaced as a global event.
///
/// This is the low-level seam between the socket loop (platform-specific) and
/// the parse/decision logic (agnostic). The std driver triggers it directly; a
/// downstream esp loop triggers the identical event after bridging the datagram
/// across silicon. The [`handle_udp_packet`] observer is the consumer.
#[derive(Debug, Clone, Event)]
pub struct UdpPacket {
	/// The datagram's sender.
	pub from: SocketAddr,
	/// The datagram's raw bytes.
	pub bytes: Vec<u8>,
}

/// A resolved mDNS service instance, one per discovered device, as a [`Component`].
///
/// The browser engine spawns an entity with this component when a service
/// instance first resolves (a usable `SRV` host+port), updates it in place when
/// the instance's host/port/addr change, and despawns the entity on an mDNS
/// goodbye (TTL=0). Downstream code queries `&MDnsService` to act on discovered
/// services, and can use `On<Add, MDnsService>` / `On<Remove, MDnsService>`
/// observers for the appear/leave edges.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct MDnsService {
	/// The service type queried, e.g. `_http._tcp.local`.
	pub service_type: SmolStr,
	/// The instance name, e.g. `My Device._http._tcp.local`.
	pub instance: SmolStr,
	/// The target host, e.g. `my-device.local` (from `SRV`).
	pub host: SmolStr,
	/// The service port (from `SRV`).
	pub port: u16,
	/// The host's IPv4 (from `A`), once known.
	pub addr: Option<Ipv4Addr>,
	/// `TXT` metadata entries, if any.
	pub txt: Vec<SmolStr>,
}

impl MDnsService {
	/// The socket address to reach this instance, once both the `A` record and
	/// `SRV` port are known.
	pub fn socket_addr(&self) -> Option<SocketAddr> {
		self.addr.map(|ip| SocketAddr::from((ip, self.port)))
	}
}

/// Config component describing a service type to browse for.
///
/// Spawn an entity carrying this (and, on the driving side, the socket) to start
/// a browser. The agnostic engine only reads the `service_type`; the
/// [`UdpEndpoint`](crate::udp::UdpEndpoint) the socket is opened from is supplied
/// by the driver ([`run_mdns_browser`] on std), keeping this component
/// platform-free.
#[derive(Debug, Clone, Component)]
pub struct MdnsBrowser {
	/// The DNS-SD service type to enumerate, e.g. `_http._tcp.local`.
	pub service_type: SmolStr,
}

impl MdnsBrowser {
	/// Browse for the given service type, e.g. `"_http._tcp.local"`.
	pub fn new(service_type: impl Into<SmolStr>) -> Self {
		Self {
			service_type: service_type.into(),
		}
	}
	/// Browse for `_http._tcp.local` (HTTP servers).
	pub fn http() -> Self { Self::new("_http._tcp.local") }
}

/// Plugin wiring the agnostic browser engine into an [`App`]: registers the
/// [`handle_udp_packet`] observer that maintains the [`MDnsService`] entities.
///
/// This is all that's needed on *any* platform; the socket driver is added
/// separately (on std via [`run_mdns_browser`]).
#[derive(Default)]
pub struct MdnsBrowserPlugin;

impl Plugin for MdnsBrowserPlugin {
	fn build(&self, app: &mut App) { app.add_observer(handle_udp_packet); }
}

/// Observer: parse an inbound [`UdpPacket`] as an mDNS response and reconcile
/// the live set of [`MDnsService`] entities (spawn / update / despawn).
///
/// Pure decision logic over the world — no IO. Correlates the `PTR`/`SRV`/`A`/
/// `TXT` records in one packet by name. A `SRV`/`A` with TTL `0` is an mDNS
/// goodbye: the matching entity is despawned. The set of service types to browse
/// is taken from every [`MdnsBrowser`] entity in the world, so a packet is only
/// acted on for types we actually asked about.
fn handle_udp_packet(
	ev: On<UdpPacket>,
	browsers: Query<&MdnsBrowser>,
	mut services: Query<(Entity, &mut MDnsService)>,
	mut commands: Commands,
) {
	let Some(response) = wire::parse_response(&ev.event().bytes) else {
		return;
	};

	// Reconcile against every browsed service type.
	for browser in browsers.iter() {
		let service_type = browser.service_type.clone();

		// Instances named directly by a PTR in this packet.
		for instance in response
			.ptr_instances(&service_type)
			.map(SmolStr::new)
			.collect::<Vec<_>>()
		{
			reconcile_instance(
				&service_type,
				&instance,
				&response,
				&mut services,
				&mut commands,
			);
		}

		// A response may carry SRV/A updates (or a goodbye) for an
		// already-known instance without re-sending its PTR. Reconcile those
		// too, by walking the instances we already track under this service
		// type whose SRV the packet touches.
		let known: Vec<SmolStr> = services
			.iter()
			.filter(|(_, svc)| svc.service_type == service_type)
			.map(|(_, svc)| svc.instance.clone())
			.collect();
		for instance in known {
			if response.srv_for(&instance).is_some() {
				reconcile_instance(
					&service_type,
					&instance,
					&response,
					&mut services,
					&mut commands,
				);
			}
		}
	}
}

/// Reconcile a single instance against one parsed response: spawn a fresh
/// [`MDnsService`] entity, update the existing one in place, or despawn it on a
/// goodbye.
fn reconcile_instance(
	service_type: &SmolStr,
	instance: &SmolStr,
	response: &wire::MdnsResponse,
	services: &mut Query<(Entity, &mut MDnsService)>,
	commands: &mut Commands,
) {
	// Locate any existing entity for this instance.
	let existing = services
		.iter()
		.find(|(_, svc)| &svc.instance == instance)
		.map(|(entity, svc)| (entity, svc.clone()));

	// A goodbye is an SRV or A record for this instance/host with TTL 0.
	let mut goodbye = false;

	// Start from the known instance (so partial updates merge) or a fresh one.
	let mut next =
		existing
			.as_ref()
			.map(|(_, svc)| svc.clone())
			.unwrap_or(MDnsService {
				service_type: service_type.clone(),
				instance: instance.clone(),
				host: SmolStr::default(),
				port: 0,
				addr: None,
				txt: Vec::new(),
			});

	if let Some(Record::Srv {
		host, port, ttl, ..
	}) = response.srv_for(instance)
	{
		if *ttl == 0 {
			goodbye = true;
		}
		next.host = host.clone();
		next.port = *port;
	}

	if !next.host.is_empty()
		&& let Some(Record::A { addr, ttl, .. }) = response.a_for(&next.host)
	{
		if *ttl == 0 {
			goodbye = true;
		}
		next.addr = Some(*addr);
	}

	if let Some(Record::Txt { entries, .. }) = response.txt_for(instance) {
		next.txt = entries.clone();
	}

	if goodbye {
		if let Some((entity, _)) = existing {
			commands.entity(entity).despawn();
		}
		return;
	}

	// We need at least a host+port (a usable SRV) before announcing.
	if next.host.is_empty() || next.port == 0 {
		return;
	}

	match existing {
		// Update an existing entity in place only if something changed.
		Some((entity, prev)) => {
			if prev != next
				&& let Ok((_, mut svc)) = services.get_mut(entity)
			{
				*svc = next;
			}
		}
		// First sighting: spawn a new service entity.
		None => {
			commands.spawn(next);
		}
	}
}

// ---------------------------------------------------------------------------
// std driver: binds the socket via the generic `UdpEndpoint`, joins multicast,
// periodically sends the PTR query, and feeds datagrams into the world. The
// only platform-specific code in this module — gated behind `udp` (the std
// runtime + `async-io`/`futures-lite`). On esp this loop lives on embassy and
// bridges into `world.trigger(UdpPacket)` instead.
// ---------------------------------------------------------------------------

/// How often the browser re-multicasts its `PTR` query, in milliseconds.
#[cfg(feature = "udp")]
pub const DEFAULT_QUERY_INTERVAL_MS: u64 = 5_000;

/// Drive an mDNS browser against `endpoint` on std, feeding inbound datagrams
/// into `world` as [`UdpPacket`] events and re-querying every
/// `query_interval_ms`.
///
/// This is the std analogue of the esp embassy loop: it binds a socket on the
/// mDNS port, joins the multicast group, sends an initial `PTR` query, then
/// concurrently (a) reads datagrams and triggers [`UdpPacket`] (which the
/// [`handle_udp_packet`] observer turns into [`MDnsService`] entities) and (b)
/// re-sends the query on an interval. It never returns on its own; spawn it as
/// an async task (e.g. via `run_async_local`).
///
/// `world` is an [`AsyncWorld`] handle so the loop can trigger events into the
/// ECS without any bridge — on std the socket runtime *is* the world runtime.
#[cfg(feature = "udp")]
pub async fn run_mdns_browser<E: crate::udp::UdpEndpoint>(
	world: AsyncWorld,
	endpoint: E,
	service_type: String,
	query_interval_ms: u64,
) -> Result {
	use super::MDNS_ENDPOINT;
	use super::MDNS_MULTICAST_V4;
	use super::MDNS_PORT;
	use core::net::Ipv4Addr;
	use core::net::SocketAddr;
	use core::net::SocketAddrV4;

	// The default mDNS configuration: bind 0.0.0.0:5353, join the group, send
	// queries to the multicast endpoint.
	let bind =
		SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MDNS_PORT));
	run_mdns_browser_on(
		world,
		endpoint,
		service_type,
		query_interval_ms,
		bind,
		MDNS_ENDPOINT,
		Some(MDNS_MULTICAST_V4),
	)
	.await
}

/// Like [`run_mdns_browser`] but with explicit `bind` / query-`target`
/// addresses and an optional multicast group to join.
///
/// [`run_mdns_browser`] is the mDNS-default specialisation of this. Exposed so a
/// host (or test) can drive the same engine over loopback / unicast without the
/// `5353` + multicast assumptions — useful where multicast is unavailable (CI).
#[cfg(feature = "udp")]
pub async fn run_mdns_browser_on<E: crate::udp::UdpEndpoint>(
	world: AsyncWorld,
	endpoint: E,
	service_type: String,
	query_interval_ms: u64,
	bind: core::net::SocketAddr,
	target: core::net::SocketAddr,
	multicast_group: Option<core::net::Ipv4Addr>,
) -> Result {
	use crate::udp::UdpSocket;
	use core::net::SocketAddr;
	use futures_lite::FutureExt;

	let socket = endpoint.bind(bind).await?;
	if let Some(group) = multicast_group {
		socket.join_multicast_v4(group).await?;
	}

	// Build the query once; it never changes for a given service type.
	let mut query = [0u8; 256];
	let query_len = wire::build_ptr_query(&mut query, &service_type)
		.ok_or_else(|| {
			bevyhow!("mDNS service type too long: {service_type}")
		})?;
	let query = &query[..query_len];

	// Initial query.
	socket.send_to(query, target).await?;
	let mut next_query = std::time::Instant::now()
		+ std::time::Duration::from_millis(query_interval_ms);

	let mut buf = vec![0u8; 1536];
	loop {
		// Race a receive against the query timer.
		let now = std::time::Instant::now();
		let until_query = next_query.saturating_duration_since(now);

		enum Wake {
			Packet(usize, SocketAddr),
			Query,
		}

		let recv = async {
			let (n, from) = socket.recv_from(&mut buf).await?;
			Result::<Wake>::Ok(Wake::Packet(n, from))
		};
		let timer = async {
			time_ext::sleep(until_query).await;
			Result::<Wake>::Ok(Wake::Query)
		};

		match recv.or(timer).await? {
			Wake::Packet(n, from) => {
				let bytes = buf[..n].to_vec();
				world
					.with(move |world: &mut World| {
						world.trigger(UdpPacket { from, bytes });
					})
					.await;
			}
			Wake::Query => {
				socket.send_to(query, target).await?;
				next_query = std::time::Instant::now()
					+ std::time::Duration::from_millis(query_interval_ms);
			}
		}
	}
}

#[cfg(test)]
#[cfg(feature = "std")]
mod test {
	use super::*;

	/// Build a browse response (PTR+SRV+A, optional TTL) for one instance.
	fn browse_response(
		service_type: &str,
		instance: &str,
		host: &str,
		port: u16,
		addr: Ipv4Addr,
		ttl: u32,
	) -> Vec<u8> {
		fn write_name(buf: &mut Vec<u8>, name: &str) {
			for part in name.split('.') {
				buf.push(part.len() as u8);
				buf.extend_from_slice(part.as_bytes());
			}
			buf.push(0);
		}
		let mut buf = Vec::new();
		buf.extend_from_slice(&0u16.to_be_bytes());
		buf.extend_from_slice(&0x8400u16.to_be_bytes());
		buf.extend_from_slice(&0u16.to_be_bytes()); // qd
		buf.extend_from_slice(&3u16.to_be_bytes()); // an: PTR,SRV,A
		buf.extend_from_slice(&0u16.to_be_bytes());
		buf.extend_from_slice(&0u16.to_be_bytes());

		write_name(&mut buf, service_type);
		buf.extend_from_slice(&wire::TYPE_PTR.to_be_bytes());
		buf.extend_from_slice(&wire::CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		let mut ptr = Vec::new();
		write_name(&mut ptr, instance);
		buf.extend_from_slice(&(ptr.len() as u16).to_be_bytes());
		buf.extend_from_slice(&ptr);

		write_name(&mut buf, instance);
		buf.extend_from_slice(&wire::TYPE_SRV.to_be_bytes());
		buf.extend_from_slice(&wire::CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		let mut srv = Vec::new();
		srv.extend_from_slice(&0u16.to_be_bytes());
		srv.extend_from_slice(&0u16.to_be_bytes());
		srv.extend_from_slice(&port.to_be_bytes());
		write_name(&mut srv, host);
		buf.extend_from_slice(&(srv.len() as u16).to_be_bytes());
		buf.extend_from_slice(&srv);

		write_name(&mut buf, host);
		buf.extend_from_slice(&wire::TYPE_A.to_be_bytes());
		buf.extend_from_slice(&wire::CLASS_IN.to_be_bytes());
		buf.extend_from_slice(&ttl.to_be_bytes());
		buf.extend_from_slice(&4u16.to_be_bytes());
		buf.extend_from_slice(&addr.octets());
		buf
	}

	/// Feed a crafted datagram through the agnostic `UdpPacket` path (no socket)
	/// and assert the [`MDnsService`] entities behave: one spawned on discovery,
	/// despawned on goodbye. This exercises the same code the std driver and the
	/// esp bridge both feed.
	#[beet_core::test]
	fn discover_and_remove_via_packet_path() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, MdnsBrowserPlugin));
		app.world_mut().spawn(MdnsBrowser::http());

		let addr = Ipv4Addr::new(192, 168, 1, 42);
		let discover = browse_response(
			"_http._tcp.local",
			"My Device._http._tcp.local",
			"my-device.local",
			8337,
			addr,
			120,
		);

		// Trigger discovery: one service entity should appear.
		app.world_mut().trigger(UdpPacket {
			from: SocketAddr::from((addr, 5353)),
			bytes: discover,
		});
		app.update();

		{
			let mut query = app.world_mut().query::<&MDnsService>();
			let found: Vec<_> = query.iter(app.world()).collect();
			found.len().xpect_eq(1);
			let svc = found[0];
			svc.instance.as_str().xpect_eq("My Device._http._tcp.local");
			svc.host.as_str().xpect_eq("my-device.local");
			svc.port.xpect_eq(8337);
			svc.addr.xpect_eq(Some(addr));
			svc.socket_addr()
				.unwrap()
				.xpect_eq(SocketAddr::from((addr, 8337)));
		}

		// A goodbye (TTL=0) despawns the entity.
		let goodbye = browse_response(
			"_http._tcp.local",
			"My Device._http._tcp.local",
			"my-device.local",
			8337,
			addr,
			0,
		);
		app.world_mut().trigger(UdpPacket {
			from: SocketAddr::from((addr, 5353)),
			bytes: goodbye,
		});
		app.update();

		let mut query = app.world_mut().query::<&MDnsService>();
		query.iter(app.world()).count().xpect_eq(0);
	}

	/// End-to-end over a real `async-io` socket on loopback (no multicast, so
	/// it's deterministic in CI): a test "responder" socket answers the
	/// browser's `PTR` query with a crafted `PTR`/`SRV`/`A` response, and we
	/// assert an [`MDnsService`] entity appears, then a goodbye despawns it.
	/// This drives the full std path: `run_mdns_browser_on` -> bind -> query ->
	/// recv -> `UdpPacket` -> observer -> entity, with the socket runtime and
	/// world runtime coincident (no bridge).
	#[cfg(feature = "udp")]
	#[beet_core::test]
	async fn browse_over_loopback_socket() {
		use crate::prelude::AsyncIoUdpEndpoint;
		use core::net::Ipv4Addr;
		use core::net::SocketAddr;
		use core::net::SocketAddrV4;

		const SERVICE: &str = "_http._tcp.local";
		const INSTANCE: &str = "My Device._http._tcp.local";
		const HOST: &str = "my-device.local";
		let addr = Ipv4Addr::new(192, 168, 1, 42);

		// A "responder" socket on a known loopback port: receives the browser's
		// query and replies to the source address it came from.
		let responder = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
		let responder_addr = responder.local_addr().unwrap();
		let responder = async_io::Async::new(responder).unwrap();

		// Run the browser inside an App on a background thread, reporting the
		// count of discovered service entities out on every update.
		let (tx, rx) = std::sync::mpsc::channel::<usize>();
		let target = responder_addr;
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AsyncPlugin, MdnsBrowserPlugin));
			app.world_mut().spawn(MdnsBrowser::http());

			let bind =
				SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
			app.add_systems(Startup, move |world: &mut World| {
				world.run_async_local(
					move |async_world: AsyncWorld| async move {
						run_mdns_browser_on(
							async_world,
							AsyncIoUdpEndpoint,
							SERVICE.to_string(),
							// re-query fast so a missed packet recovers
							200,
							bind,
							target,
							None,
						)
						.await
					},
				);
			});

			app.add_systems(Update, move |services: Query<&MDnsService>| {
				let _ = tx.send(services.iter().count());
			});
			app.run();
		});

		// Wait for the browser's query, then reply with a browse response.
		let mut qbuf = [0u8; 512];
		let (_n, browser_src) = responder.recv_from(&mut qbuf).await.unwrap();
		let discover =
			browse_response(SERVICE, INSTANCE, HOST, 8337, addr, 120);
		responder.send_to(&discover, browser_src).await.unwrap();

		// Poll the reported count until the instance shows up.
		wait_for(&rx, |count| *count == 1).await;

		// Send a goodbye (TTL=0) and assert removal.
		let goodbye = browse_response(SERVICE, INSTANCE, HOST, 8337, addr, 0);
		responder.send_to(&goodbye, browser_src).await.unwrap();
		wait_for(&rx, |count| *count == 0).await;
	}

	/// Poll the counts coming over `rx` until `pred` holds, with a generous
	/// timeout so a flaky datagram doesn't hang CI.
	#[cfg(feature = "udp")]
	async fn wait_for(
		rx: &std::sync::mpsc::Receiver<usize>,
		pred: impl Fn(&usize) -> bool,
	) {
		let deadline =
			std::time::Instant::now() + std::time::Duration::from_secs(10);
		loop {
			while let Ok(snapshot) = rx.try_recv() {
				if pred(&snapshot) {
					return;
				}
			}
			if std::time::Instant::now() > deadline {
				panic!("timed out waiting for browser state");
			}
			time_ext::sleep_millis(20).await;
		}
	}
}
