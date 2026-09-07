//! Raw analytics segment objects: their keyspace and gzip NDJSON codec.
use crate::prelude::*;
use beet_core::prelude::*;

/// Represents one immutable batch of raw analytics events in a [`BlobStore`].
///
/// Writers own a UUID namespace and append fresh objects beneath the event day,
/// so any number of processes can ingest concurrently without reading or
/// contending on an existing object. The daily rollup compacts these objects
/// into an [`AnalyticsArchive`] and then removes them.
pub struct AnalyticsSegment;

impl AnalyticsSegment {
	/// The prefix containing uncompacted raw event batches.
	pub const PREFIX: &'static str = "analytics/raw/segments";

	/// Returns the path for one writer's batch.
	///
	/// Paths have the form
	/// `analytics/raw/segments/2026-08-01/<writer>/<timestamp>-<sequence>.ndjson.gz`.
	pub fn object_path(
		date: &str,
		writer: Uuid,
		timestamp: u64,
		sequence: u64,
	) -> SmolPath {
		SmolPath::new(format!(
			"{}/{date}/{writer}/{timestamp}-{sequence}.ndjson.gz",
			Self::PREFIX
		))
	}

	/// Returns the UTC date encoded in a segment path.
	pub(crate) fn date(path: &SmolPath) -> Option<SmolStr> {
		let remainder = path
			.as_str()
			.strip_prefix(Self::PREFIX)?
			.strip_prefix('/')?;
		let mut parts = remainder.split('/');
		let date = parts.next()?;
		let writer = parts.next()?;
		let object = parts.next()?;
		let (timestamp, sequence) =
			object.strip_suffix(".ndjson.gz")?.split_once('-')?;
		(parts.next().is_none()
			&& Timestamp::parse_date(date).is_some()
			&& writer.parse::<Uuid>().is_ok()
			&& timestamp.parse::<u64>().is_ok()
			&& sequence.parse::<u64>().is_ok())
		.then(|| date.into())
	}

	/// Writes and read-verifies one segment object.
	pub(crate) async fn write(
		store: &BlobStore,
		path: SmolPath,
		events: &[AnalyticsEvent],
	) -> Result<SmolPath> {
		let bytes = AnalyticsArchive::encode(events)?;
		store.insert(&path, bytes.clone()).await?;
		let actual = store.get(&path).await?;
		if actual != bytes {
			bevybail!(
				"analytics segment `{path}` was written to {} but failed read-back verification",
				store.describe()
			);
		}
		path.xok()
	}

	/// Reads the segments of every date `wanted` accepts, failing if any of them
	/// cannot be read or decoded and leaving the rest untouched in the store.
	///
	/// Compaction only ever consumes complete days, and the current day is the
	/// bulk of the keyspace (a busy day is hundreds of objects), so the filter
	/// is applied to paths rather than to decoded events: an incomplete day is
	/// never fetched at all, which is also what lets a half-written segment
	/// under today's date sit there without failing tonight's run.
	pub(crate) async fn read_dates(
		store: &BlobStore,
		wanted: impl Fn(&SmolStr) -> bool,
	) -> Result<Vec<(SmolPath, Vec<AnalyticsEvent>)>> {
		Self::dated_paths(store)
			.await?
			.into_iter()
			.filter(|(_, date)| wanted(date))
			.map(async |(path, _)| {
				let events = Self::read(store, &path).await?;
				Ok::<_, BevyError>((path, events))
			})
			.xmap(|reads| {
				async_ext::try_join_all_bounded(
					BlobStore::GET_ALL_CONCURRENCY,
					reads,
				)
			})
			.await
	}

	/// Returns every date holding at least one segment, in ascending order.
	pub(crate) async fn dates(store: &BlobStore) -> Result<Vec<SmolStr>> {
		Self::dated_paths(store)
			.await?
			.into_iter()
			.map(|(_, date)| date)
			.collect::<HashSet<_>>()
			.into_iter()
			.collect::<Vec<_>>()
			.xmap(|mut dates| {
				dates.sort();
				dates
			})
			.xok()
	}

	/// Reads every valid segment, warning and skipping unreadable objects.
	pub(crate) async fn read_all_lossy(
		store: &BlobStore,
	) -> Result<Vec<(SmolPath, Vec<AnalyticsEvent>)>> {
		Self::dated_paths(store)
			.await?
			.into_iter()
			.map(async |(path, _)| {
				let result = Self::read(store, &path).await;
				(path, result)
			})
			.xmap(|reads| {
				async_ext::join_all_bounded(
					BlobStore::GET_ALL_CONCURRENCY,
					reads,
				)
			})
			.await
			.into_iter()
			.filter_map(|(path, result)| match result {
				Ok(events) => Some((path, events)),
				Err(err) => {
					warn!(
						"skipping unreadable analytics segment {path}: {err}"
					);
					None
				}
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Reads and validates one segment against the date encoded in its path.
	async fn read(
		store: &BlobStore,
		path: &SmolPath,
	) -> Result<Vec<AnalyticsEvent>> {
		let Some(date) = Self::date(path) else {
			bevybail!("invalid analytics segment path `{path}`");
		};
		let events = AnalyticsArchive::decode(&store.get(path).await?)?;
		for event in &events {
			if event.date() != date {
				bevybail!(
					"analytics segment `{path}` contains event {} for {}, not {date}",
					event.id,
					event.date()
				);
			}
		}
		events.xok()
	}

	/// Returns every segment path beside the date it encodes, in deterministic
	/// order. Anything else in the store belongs to another writer and is left
	/// alone rather than treated as a malformed segment.
	async fn dated_paths(
		store: &BlobStore,
	) -> Result<Vec<(SmolPath, SmolStr)>> {
		if !store.store_exists().await? {
			return Vec::new().xok();
		}
		let mut paths = store
			.list()
			.await?
			.into_iter()
			.filter_map(|path| Self::date(&path).map(|date| (path, date)))
			.collect::<Vec<_>>();
		paths.sort();
		paths.xok()
	}
}
