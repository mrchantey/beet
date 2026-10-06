//! What a server lets one client make it allocate.
use beet_core::prelude::*;

/// The caps every connection a server accepts is served under, so peak memory
/// is a property of the declaration rather than of the traffic.
///
/// These are remote-OOM guards, not tuning knobs. A page build holds megabytes
/// for as long as it runs, so an unbounded accept loop spends memory linearly in
/// whatever concurrency a client chooses, and a body read sized from the
/// client's own `content-length` spends it on a single header. Live, `beet.org`
/// (1913 MB, no swap) was killed by the first: around a hundred concurrent
/// requests for one unmatched path, which is what an ordinary vulnerability
/// scanner does by default.
///
/// Declared on the server, so one process can serve a public listener and a
/// loopback one under different caps:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # let mut world = World::new();
/// world.spawn((HttpServer::default(), ServerLimits {
///     max_in_flight: 4,
///     ..default()
/// }));
/// ```
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ServerLimits {
	/// How many accepted requests this server dispatches at once. Beyond it a
	/// connection waits for a permit, so a burst queues instead of being
	/// answered all at once, and peak memory is this times the cost of the
	/// dearest route.
	///
	/// Waiting costs a connection its socket and its header buffer alone: the
	/// permit is taken around the dispatch, not around the connection, so a
	/// client slow to send its request line holds nothing back.
	///
	/// Lower is faster as well as lighter. Beet builds a page on the one world
	/// thread, so concurrency past a handful buys queueing rather than
	/// throughput while multiplying the memory held at once: 200 concurrent
	/// requests for `beet.org`'s dearest route peak at 814 MB over baseline and
	/// take 17.3 s at 32, against 400 MB and 5.8 s at 16.
	pub max_in_flight: usize,
	/// The largest request body this server reads, in bytes. A `content-length`
	/// above it is answered `413` and the connection closed, without reading or
	/// allocating a byte of the body.
	pub max_body_bytes: usize,
	/// How long a request waits for a permit before being answered `503`.
	///
	/// The valve on the one way [`max_in_flight`](Self::max_in_flight) could
	/// wedge a server: a handler that calls its OWN server holds a permit while
	/// it waits for a second one (the charcell image fetch reaches the process's
	/// canonical loopback port exactly like a remote client), so enough nested
	/// renders at once would deadlock. A wait that expires answers rather than
	/// hanging, which turns that into a reported slow burst instead of a server
	/// that never recovers.
	pub dispatch_timeout: Duration,
}

impl Default for ServerLimits {
	fn default() -> Self {
		Self {
			// bounds the worst case to a few hundred MB over baseline on the
			// smallest box beet deploys to (1913 MB, no swap), while staying
			// well clear of the nesting depth a self-calling handler needs
			max_in_flight: 16,
			max_body_bytes: 8 * 1024 * 1024,
			// long enough to outlast a legitimate burst queueing behind the cap,
			// short enough that a wedge reports itself
			dispatch_timeout: Duration::from_secs(30),
		}
	}
}

/// A server's live limits: the [`ServerLimits`] it resolved at start, and the
/// permits its connections share. Built once per accept loop and cloned per
/// connection.
#[cfg(all(
	any(feature = "hyper", feature = "server"),
	not(target_arch = "wasm32")
))]
#[derive(Clone)]
pub(crate) struct ServerGuard {
	limits: ServerLimits,
	dispatch: std::sync::Arc<async_lock::Semaphore>,
}

#[cfg(all(
	any(feature = "hyper", feature = "server"),
	not(target_arch = "wasm32")
))]
impl ServerGuard {
	/// The limits declared on `entity` (the [`HttpServer`] requirement puts the
	/// defaults there when nothing else does), with a permit pool to match.
	pub(crate) async fn resolve(entity: &AsyncEntity) -> Result<Self> {
		let limits = entity
			.with(|entity: EntityWorldMut| {
				entity.get::<ServerLimits>().copied().unwrap_or_default()
			})
			.await?;
		Self {
			dispatch: std::sync::Arc::new(async_lock::Semaphore::new(
				limits.max_in_flight,
			)),
			limits,
		}
		.xok()
	}

	/// The largest request body this server reads.
	pub(crate) fn max_body_bytes(&self) -> usize { self.limits.max_body_bytes }

	/// Wait for a dispatch permit, held until the returned guard drops, or
	/// `None` once [`ServerLimits::dispatch_timeout`] has passed.
	pub(crate) async fn dispatch(
		&self,
	) -> Option<async_lock::SemaphoreGuardArc> {
		async_ext::timeout(
			self.limits.dispatch_timeout,
			self.dispatch.clone().acquire_arc(),
		)
		.await
		.ok()
	}
}
