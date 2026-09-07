//! Buffered analytics persistence over [`BlobStore`].
use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// A buffered raw analytics writer backed solely by a [`BlobStore`].
///
/// Each clone shares one in-process buffer and one writer UUID. Events flush to
/// immutable gzip NDJSON segments when the buffer reaches its age or byte limit,
/// and the analytics plugin gates [`AppExit`] until every remaining event has
/// flushed. Fresh writer/timestamp keys make concurrent processes contention-free.
#[derive(Clone, Deref, Component)]
pub struct AnalyticsStore {
	/// The resolved raw-segment store.
	#[deref]
	pub store: BlobStore,
	writer: Uuid,
	buffer: Arc<Mutex<AnalyticsBuffer>>,
}

impl AnalyticsStore {
	/// Create a writer over `store` with the default buffer limits.
	pub fn new(store: BlobStore) -> Self {
		Self::with_config(store, &AnalyticsConfig::default())
	}

	/// Create a writer over `store` with `config`'s buffer limits.
	pub fn with_config(store: BlobStore, config: &AnalyticsConfig) -> Self {
		Self {
			store,
			writer: uuid_ext::now_v7(),
			buffer: Arc::new(Mutex::new(AnalyticsBuffer::new(
				config.segment_max_age,
				config.segment_max_bytes,
			))),
		}
	}

	/// The local analytics store at `dir`, as used by `beet analytics`.
	#[cfg(feature = "fs")]
	pub fn local(dir: AbsPathBuf) -> Self {
		Self::new(BlobStore::new(FsStore::new(dir)))
	}

	/// The remote S3 analytics store at `bucket_name`, using the SDK's default
	/// region provider chain. Errors without the native `aws_sdk` backend.
	pub fn remote(bucket_name: &str) -> Result<Self> {
		Self::new(BlobStore::remote(bucket_name)?).xok()
	}

	/// Buffers an event, flushing when the configured size is reached.
	///
	/// A failed write restores every unwritten event to the buffer so a later age,
	/// size, or shutdown flush can retry it.
	pub async fn record(&self, event: AnalyticsEvent) -> Result {
		let encoded_len = Self::encoded_len(&event)?;
		let events = self.buffer.lock().unwrap().push(event, encoded_len);
		match events {
			Some(events) => self.write(events).await,
			None => Ok(()),
		}
	}

	/// Flush every buffered event, doing nothing when the buffer is empty.
	pub async fn flush(&self) -> Result {
		let events = self.buffer.lock().unwrap().take();
		match events {
			Some(events) => self.write(events).await,
			None => Ok(()),
		}
	}

	/// Read every raw segment, deduplicating repeated event ids to their newest
	/// timestamp and skipping unreadable segment objects with a warning.
	pub async fn read_all_lossy(&self) -> Result<Vec<AnalyticsEvent>> {
		let segments = AnalyticsSegment::read_all_lossy(&self.store).await?;
		Self::dedupe(segments.into_iter().flat_map(|(_, events)| events)).xok()
	}

	/// Take the buffer only when its oldest event reached the age limit.
	fn take_aged(&self) -> Option<Vec<AnalyticsEvent>> {
		self.buffer.lock().unwrap().take_aged()
	}

	/// Writes a drained buffer as one segment per event day.
	async fn write(&self, events: Vec<AnalyticsEvent>) -> Result {
		let mut by_date = HashMap::<SmolStr, Vec<AnalyticsEvent>>::default();
		for event in events {
			by_date.entry(event.date()).or_default().push(event);
		}
		let mut days = by_date.into_iter().collect::<Vec<_>>();
		days.sort_by(|left, right| left.0.cmp(&right.0));
		for (index, (date, events)) in days.iter().enumerate() {
			let (timestamp, sequence) = {
				let mut buffer = self.buffer.lock().unwrap();
				buffer.next_sequence += 1;
				(analytics_ext::now_ms(), buffer.next_sequence)
			};
			let path = AnalyticsSegment::object_path(
				date,
				self.writer,
				timestamp,
				sequence,
			);
			if let Err(err) =
				AnalyticsSegment::write(&self.store, path, events).await
			{
				let pending = days[index..]
					.iter()
					.flat_map(|(_, events)| events.iter().cloned())
					.collect::<Vec<_>>();
				let bytes = pending.iter().try_fold(0, |bytes, event| {
					Ok::<_, BevyError>(bytes + Self::encoded_len(event)?)
				})?;
				self.buffer.lock().unwrap().restore(pending, bytes);
				bevybail!(
					"analytics: could not write a segment to {}; unwritten events remain buffered.\n{err}",
					self.store.describe()
				);
			}
		}
		Ok(())
	}

	/// Returns one event's uncompressed NDJSON size.
	fn encoded_len(event: &AnalyticsEvent) -> Result<usize> {
		Ok(serde_json::to_vec(event)?.len() + 1)
	}

	/// Keep the latest occurrence of each event id, sorted by id.
	pub(crate) fn dedupe(
		events: impl IntoIterator<Item = AnalyticsEvent>,
	) -> Vec<AnalyticsEvent> {
		let mut by_id = HashMap::<Uuid, AnalyticsEvent>::default();
		for event in events {
			match by_id.get(&event.id) {
				Some(existing) if existing.timestamp > event.timestamp => {}
				_ => {
					by_id.insert(event.id, event);
				}
			}
		}
		let mut events = by_id.into_values().collect::<Vec<_>>();
		events.sort_by_key(|event| event.id);
		events
	}
}

/// Mutable state shared by every clone of one process writer.
struct AnalyticsBuffer {
	events: Vec<AnalyticsEvent>,
	bytes: usize,
	opened: Option<Instant>,
	max_age: Duration,
	max_bytes: usize,
	next_sequence: u64,
}

impl AnalyticsBuffer {
	fn new(max_age: Duration, max_bytes: usize) -> Self {
		Self {
			events: Vec::new(),
			bytes: 0,
			opened: None,
			max_age,
			max_bytes: max_bytes.max(1),
			next_sequence: 0,
		}
	}

	fn push(
		&mut self,
		event: AnalyticsEvent,
		encoded_len: usize,
	) -> Option<Vec<AnalyticsEvent>> {
		self.opened.get_or_insert_with(Instant::now);
		self.bytes += encoded_len;
		self.events.push(event);
		(self.bytes >= self.max_bytes)
			.then(|| self.take())
			.flatten()
	}

	fn take_aged(&mut self) -> Option<Vec<AnalyticsEvent>> {
		self.opened
			.filter(|opened| opened.elapsed() >= self.max_age)
			.and_then(|_| self.take())
	}

	fn take(&mut self) -> Option<Vec<AnalyticsEvent>> {
		if self.events.is_empty() {
			return None;
		}
		self.bytes = 0;
		self.opened = None;
		Some(core::mem::take(&mut self.events))
	}

	fn restore(&mut self, mut events: Vec<AnalyticsEvent>, bytes: usize) {
		self.opened.get_or_insert_with(Instant::now);
		self.bytes += bytes;
		events.append(&mut self.events);
		self.events = events;
	}
}

/// Observer: resolve the [`BlobStore`] named beside an [`AnalyticsConfig`] and
/// attach its buffered writer to that config entity.
pub(super) fn spawn_store_on_config(
	ev: On<Add, AnalyticsConfig>,
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	if stores.contains(ev.entity) {
		return;
	}
	let entity = ev.entity;
	commands.run(async move |world| {
		let config_entity = world.entity(entity);
		if config_entity.get::<AnalyticsStore, _>(|_| ()).await.is_ok() {
			return Ok(());
		}
		let Ok(target) = config_entity
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await
		else {
			bevybail!(
				"an `AnalyticsConfig` records to the store it names: add a `StoreRef` beside it pointing at a blob-store declaration, ie `<S3BucketBlock bx:ref=\"analytics\" label=\"analytics\" runtime_write=true deploy_versioned=false/>` and `{{(AnalyticsConfig, StoreRef($analytics))}}`"
			);
		};
		let config = config_entity
			.get::<AnalyticsConfig, _>(Clone::clone)
			.await?;
		let store = StoreRef::resolve::<BlobStore>(&world, target).await?;
		config_entity
			.insert(AnalyticsStore::with_config(store, &config))
			.await?;
		Ok(())
	});
}

/// Observer: send every emitted event to each configured buffered writer.
pub(super) fn handle_analytics_event(
	ev: On<AnalyticsEvent>,
	stores: Query<&AnalyticsStore, With<AnalyticsConfig>>,
	commands: AsyncCommands,
) {
	let stores = stores.iter().cloned().collect::<Vec<_>>();
	if stores.is_empty() {
		return;
	}
	let event = ev.event().clone();
	commands.run(async move |_| {
		for store in stores {
			store.record(event.clone()).await?;
		}
		Ok(())
	});
}

/// Flush buffers whose oldest event reached its configured age.
pub(super) fn flush_aged_buffers(
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	for store in stores.iter() {
		let Some(events) = store.take_aged() else {
			continue;
		};
		let store = store.clone();
		commands.run(async move |_| store.write(events).await);
	}
}

/// State for the one exit signal currently waiting on analytics flushes.
#[derive(Default, Resource)]
pub(super) struct AnalyticsShutdown {
	exit: Option<AppExit>,
	flushing: bool,
	ready: bool,
}

/// Delay process exit until every in-process analytics buffer has flushed.
pub(super) fn flush_on_exit(
	mut exits: ResMut<Messages<AppExit>>,
	mut shutdown: ResMut<AnalyticsShutdown>,
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	if shutdown.ready || stores.is_empty() {
		return;
	}
	for candidate in exits.drain() {
		if shutdown.exit.is_none() || candidate.is_error() {
			shutdown.exit = Some(candidate);
		}
	}
	if shutdown.flushing || shutdown.exit.is_none() {
		return;
	}
	shutdown.flushing = true;
	let stores = stores.iter().cloned().collect::<Vec<_>>();
	commands.run(async move |world| {
		for store in stores {
			if let Err(err) = store.flush().await {
				world.handle_command_error::<AnalyticsShutdown>(err).await;
			}
		}
		let exit = world
			.with_resource::<AnalyticsShutdown, _>(|mut shutdown| {
				shutdown.ready = true;
				shutdown.exit.take().unwrap_or(AppExit::Success)
			})
			.await;
		world.write_message(exit).await;
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::exports::bytes::Bytes;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	fn request(path: &str) -> AnalyticsEvent {
		AnalyticsEvent::new(path, AnalyticsEventData::Request {
			status: 200,
			method: "GET".into(),
			user_agent: None,
			referrer: None,
		})
	}

	fn immediate_config() -> AnalyticsConfig {
		AnalyticsConfig {
			segment_max_bytes: 1,
			..default()
		}
	}

	fn config_with_store(
		world: &mut World,
		store: impl Bundle,
	) -> (Entity, Entity) {
		let store = world.spawn(store).flush();
		let config = world.spawn((immediate_config(), StoreRef(store))).flush();
		(config, store)
	}

	#[beet_core::test]
	async fn size_flush_roundtrips_a_segment() {
		let store = BlobStore::temp();
		let analytics = AnalyticsStore::with_config(store, &immediate_config());
		let event = request("/about");
		let id = event.id;
		analytics.record(event).await.unwrap();
		let loaded = analytics.read_all_lossy().await.unwrap();
		loaded.len().xpect_eq(1);
		loaded[0].id.xpect_eq(id);
		loaded[0].path.as_str().xpect_eq("/about");
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn fs_segments_are_multi_writer_safe() {
		let dir = AbsPathBuf::new_workspace_rel(
			"target/tests/beet_net/analytics-multi-writer",
		)
		.unwrap();
		fs_ext::remove(&dir).ok();
		let store = BlobStore::new(FsStore::new(dir));
		let left =
			AnalyticsStore::with_config(store.clone(), &immediate_config());
		let right =
			AnalyticsStore::with_config(store.clone(), &immediate_config());
		left.record(request("/left")).await.unwrap();
		right.record(request("/right")).await.unwrap();
		let paths = store.list().await.unwrap();
		paths.len().xpect_eq(2);
		for writer in [left.writer, right.writer] {
			paths
				.iter()
				.any(|path| path.as_str().contains(&writer.to_string()))
				.xpect_true();
		}
		paths[0].xpect_not_eq(paths[1].clone());
	}

	#[beet_core::test]
	async fn age_flushes_without_another_event() {
		let config = AnalyticsConfig {
			segment_max_age: Duration::ZERO,
			segment_max_bytes: usize::MAX,
			..default()
		};
		let store = BlobStore::temp();
		let analytics = AnalyticsStore::with_config(store.clone(), &config);
		analytics.record(request("/aged")).await.unwrap();
		let mut world = (AsyncPlugin,).into_world();
		world.spawn(analytics);
		world.run_system_once(flush_aged_buffers).unwrap();
		AsyncRunner::settle_async_tasks(&mut world).await;
		AnalyticsSegment::read_dates(&store, |_| true)
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn exit_waits_for_the_last_buffer() {
		let config = AnalyticsConfig {
			segment_max_bytes: usize::MAX,
			..default()
		};
		let store = BlobStore::temp();
		let analytics = AnalyticsStore::with_config(store.clone(), &config);
		analytics.record(request("/shutdown")).await.unwrap();
		let mut world = (AsyncPlugin,).into_world();
		world.init_resource::<Messages<AppExit>>();
		world.init_resource::<AnalyticsShutdown>();
		world.spawn(analytics);
		world.write_message(AppExit::Success);
		world.run_system_once(flush_on_exit).unwrap();
		world.should_exit().xpect_none();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.should_exit().xpect_some();
		AnalyticsSegment::read_dates(&store, |_| true)
			.await
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn config_entity_carries_the_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (config, _) = config_with_store(&mut world, InMemoryStore::new());
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.entity(config)
			.contains::<AnalyticsStore>()
			.xpect_true();
	}

	#[beet_core::test]
	async fn records_to_the_declared_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (_, store_entity) =
			config_with_store(&mut world, InMemoryStore::new());
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.trigger(request("/about"));
		AsyncRunner::settle_async_tasks(&mut world).await;
		let store = world
			.entity(store_entity)
			.get::<BlobStore>()
			.unwrap()
			.clone();
		AnalyticsSegment::read_dates(&store, |_| true)
			.await
			.unwrap()
			.into_iter()
			.flat_map(|(_, events)| events)
			.next()
			.unwrap()
			.path
			.as_str()
			.xpect_eq("/about");
	}

	static RAISED: std::sync::Mutex<Vec<String>> =
		std::sync::Mutex::new(Vec::new());

	fn collect_error(err: BevyError, _cx: bevy::ecs::error::ErrorContext) {
		RAISED.lock().unwrap().push(err.to_string());
	}

	fn collecting_world() -> World {
		RAISED.lock().unwrap().clear();
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		world.insert_resource(bevy::ecs::error::FallbackErrorHandler(
			collect_error,
		));
		world
	}

	fn raised() -> Vec<String> { RAISED.lock().unwrap().clone() }

	#[beet_core::test]
	async fn analytics_failures_are_loud() {
		let mut world = collecting_world();
		world.spawn(AnalyticsConfig::default()).flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		raised().join("\n").xpect_contains("StoreRef");

		let mut world = collecting_world();
		let attempts = Arc::new(AtomicUsize::new(0));
		config_with_store(&mut world, FailingStore {
			attempts: attempts.clone(),
		});
		AsyncRunner::settle_async_tasks(&mut world).await;
		for index in 0..8 {
			world.trigger(request(&format!("/page-{index}")));
			AsyncRunner::settle_async_tasks(&mut world).await;
		}
		let raised = raised();
		raised.len().xpect_eq(8);
		raised[0]
			.as_str()
			.xpect_contains("could not write a segment to")
			.xpect_contains("unwritten events remain buffered")
			.xpect_contains("no-such-store");
		attempts.load(Ordering::SeqCst).xpect_eq(8);
	}

	#[derive(Clone, Component)]
	#[component(on_add = BlobStore::on_add::<Self>)]
	struct FailingStore {
		attempts: Arc<AtomicUsize>,
	}

	impl BlobStoreProvider for FailingStore {
		fn box_clone(&self) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn with_subdir(&self, _path: SmolPath) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn id(&self) -> &'static str { "failing" }
		fn root_key(&self) -> SmolStr { "no-such-store".into() }
		fn region(&self) -> Option<String> { None }
		fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { true.xok() })
		}
		fn store_create(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn store_remove(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn insert(
			&self,
			_path: &SmolPath,
			_body: Bytes,
		) -> SendBoxedFuture<Result> {
			self.attempts.fetch_add(1, Ordering::SeqCst);
			Box::pin(async { bevybail!("store unavailable") })
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
			Box::pin(async { Vec::new().xok() })
		}
		fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
			let path = path.clone();
			Box::pin(async move { bevybail!("no object at {path}") })
		}
		fn exists(&self, _path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { false.xok() })
		}
		fn remove(&self, _path: &SmolPath) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn public_url(
			&self,
			_path: &SmolPath,
		) -> SendBoxedFuture<Result<Option<String>>> {
			Box::pin(async { None.xok() })
		}
	}
}
