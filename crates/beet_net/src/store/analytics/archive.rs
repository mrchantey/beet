//! Daily analytics archives, JSONL through the shared [`Jsonl`] codec.
use crate::exports::bytes::Bytes;
use crate::prelude::*;
use beet_core::prelude::*;

/// Represents one day's durable, compacted raw analytics events.
///
/// Daily objects make retries deterministic: every compaction of a day replaces
/// the same object after merging its prior archive with newly arrived segments.
/// JSONL keeps the format streamable even though today's small-volume reader
/// decodes a whole day at once.
pub struct AnalyticsArchive;

impl AnalyticsArchive {
	/// The prefix analytics owns in an archive store.
	pub const PREFIX: &'static str = "analytics/raw";

	/// Returns the object path for one UTC date, under
	/// [`AnalyticsSegment::CODEC`] like every raw analytics object.
	pub fn object_path(date: &str) -> RelPath {
		RelPath::new(format!(
			"{}/{date}.{}",
			Self::PREFIX,
			AnalyticsSegment::CODEC.extension()
		))
	}

	/// Returns the UTC date encoded in a daily archive path. An object under
	/// any other name, another codec's included, is another writer's.
	pub(crate) fn date(path: &RelPath) -> Option<SmolStr> {
		let date = path
			.as_str()
			.strip_prefix(Self::PREFIX)?
			.strip_prefix('/')?
			.strip_suffix(AnalyticsSegment::CODEC.extension())?
			.strip_suffix('.')?;
		(!date.contains('/') && Timestamp::parse_date(date).is_some())
			.then(|| date.into())
	}

	/// Encodes events as deterministic JSONL ordered by event ID, at the
	/// archive-grade level: written once a night, kept for good.
	pub fn encode(events: &[AnalyticsEvent]) -> Result<Bytes> {
		Jsonl::encode(
			AnalyticsSegment::CODEC,
			JsonlLevel::Best,
			AnalyticsEvent::by_id(events),
		)
	}

	/// Reads and validates a daily archive when it exists.
	pub(crate) async fn read(
		store: &BlobStore,
		date: &str,
	) -> Result<Option<Vec<AnalyticsEvent>>> {
		let path = Self::object_path(date);
		if !store.exists(&path).await? {
			return None.xok();
		}
		let events = Jsonl::decode::<AnalyticsEvent>(
			AnalyticsSegment::CODEC,
			&store.get(&path).await?,
		)?;
		for event in &events {
			if event.date() != date {
				bevybail!(
					"analytics archive `{path}` contains event {} for {}, not {date}",
					event.id,
					event.date()
				);
			}
		}
		Some(events).xok()
	}

	/// Returns every archived UTC date in deterministic order.
	pub(crate) async fn dates(store: &BlobStore) -> Result<Vec<SmolStr>> {
		if !store.store_exists().await? {
			return Vec::new().xok();
		}
		let mut dates = store
			.list()
			.await?
			.into_iter()
			.filter_map(|path| Self::date(&path))
			.collect::<Vec<_>>();
		dates.sort();
		dates.dedup();
		dates.xok()
	}

	/// Writes a daily archive and verifies its exact bytes by reading it back.
	pub async fn write(
		store: &BlobStore,
		date: &str,
		events: &[AnalyticsEvent],
	) -> Result<RelPath> {
		let path = Self::object_path(date);
		let expected = Self::encode(events)?;
		store.insert(&path, expected.clone()).await?;
		let actual = store.get(&path).await?;
		if actual != expected {
			bevybail!(
				"analytics archive `{path}` was written to {} but failed read-back verification",
				store.describe()
			);
		}
		path.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn events() -> Vec<AnalyticsEvent> {
		["/", "/docs", "/docs/intro"]
			.into_iter()
			.map(|path| {
				AnalyticsEvent::new(path, AnalyticsEventData::PageView {
					duration_ms: 4200,
					referrer: Some("https://beet.org".into()),
					title: Some("Beet".into()),
					client: default(),
				})
			})
			.collect()
	}

	fn paths<'a>(
		events: impl IntoIterator<Item = &'a AnalyticsEvent>,
	) -> Vec<SmolStr> {
		events.into_iter().map(|event| event.path.clone()).collect()
	}

	/// The archive is lossless and its bytes are a pure function of the day: it
	/// is the primary copy once compaction deletes the segments, and a re-run
	/// must overwrite its own object rather than write a different one.
	#[beet_core::test]
	fn round_trips_a_day() {
		let events = events();
		let bytes = AnalyticsArchive::encode(&events).unwrap();
		// zstd, not the json it holds
		bytes[..4].to_vec().xpect_eq(vec![0x28, 0xb5, 0x2f, 0xfd]);
		AnalyticsArchive::encode(&events)
			.unwrap()
			.xpect_eq(bytes.clone());
		// ..and the order events arrive in does not change the object
		let mut shuffled = events.clone();
		shuffled.reverse();
		AnalyticsArchive::encode(&shuffled)
			.unwrap()
			.xpect_eq(bytes.clone());

		let path = AnalyticsArchive::object_path("2026-08-01");
		path.to_string()
			.xpect_eq("analytics/raw/2026-08-01.jsonl.zst");
		AnalyticsArchive::date(&path).xpect_eq(Some("2026-08-01".into()));
		// another codec's object is another writer's, not a day
		AnalyticsArchive::date(&RelPath::new(
			"analytics/raw/2026-08-01.jsonl.gz",
		))
		.xpect_eq(None);
		AnalyticsArchive::date(&RelPath::new("analytics/raw/2026-08-01.json"))
			.xpect_eq(None);

		let decoded =
			Jsonl::decode_path::<AnalyticsEvent>(&path, &bytes).unwrap();
		decoded.len().xpect_eq(3);
		paths(AnalyticsEvent::by_id(&decoded))
			.xpect_eq(paths(AnalyticsEvent::by_id(&events)));
	}

	/// The read-back is the point: segments are only deleted because this object
	/// is known to hold their bytes, and a store that accepted a write it did
	/// not keep is exactly the failure that would make that unsafe.
	#[beet_core::test]
	async fn writes_and_reads_back_the_exact_object() {
		let store = BlobStore::temp();
		let events = events();
		let path = AnalyticsArchive::write(&store, &events[0].date(), &events)
			.await
			.unwrap();
		Jsonl::decode_path::<AnalyticsEvent>(
			&path,
			&store.get(&path).await.unwrap(),
		)
		.unwrap()
		.len()
		.xpect_eq(3);
		AnalyticsArchive::read(&store, &events[0].date())
			.await
			.unwrap()
			.unwrap()
			.len()
			.xpect_eq(3);
	}
}
