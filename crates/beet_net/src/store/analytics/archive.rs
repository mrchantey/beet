//! Daily analytics archives and the gzip NDJSON codec shared with segments.
use crate::exports::bytes::Bytes;
use crate::prelude::*;
use beet_core::prelude::*;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::Read;
use std::io::Write;

/// Represents one day's durable, compacted raw analytics events.
///
/// Daily objects make retries deterministic: every compaction of a day replaces
/// the same object after merging its prior archive with newly arrived segments.
/// NDJSON keeps the format streamable even though today's small-volume reader
/// decodes a whole day at once.
pub struct AnalyticsArchive;

impl AnalyticsArchive {
	/// The prefix analytics owns in an archive store.
	pub const PREFIX: &'static str = "analytics/raw";

	/// Returns the object path for one UTC date.
	pub fn object_path(date: &str) -> SmolPath {
		SmolPath::new(format!("{}/{date}.ndjson.gz", Self::PREFIX))
	}

	/// Returns the UTC date encoded in a daily archive path.
	pub(crate) fn date(path: &SmolPath) -> Option<SmolStr> {
		let date = path
			.as_str()
			.strip_prefix(Self::PREFIX)?
			.strip_prefix('/')?
			.strip_suffix(".ndjson.gz")?;
		(!date.contains('/') && Timestamp::parse_date(date).is_some())
			.then(|| date.into())
	}

	/// Encodes events as deterministic gzip NDJSON ordered by event ID.
	pub fn encode(events: &[AnalyticsEvent]) -> Result<Bytes> {
		let mut events = events.iter().collect::<Vec<_>>();
		events.sort_by_key(|event| event.id);
		let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
		for event in events {
			encoder.write_all(&serde_json::to_vec(event)?)?;
			encoder.write_all(b"\n")?;
		}
		Bytes::from(encoder.finish()?).xok()
	}

	/// Decodes gzip NDJSON into analytics events.
	pub fn decode(bytes: &[u8]) -> Result<Vec<AnalyticsEvent>> {
		let mut ndjson = String::new();
		GzDecoder::new(bytes).read_to_string(&mut ndjson)?;
		ndjson
			.lines()
			.filter(|line| !line.trim().is_empty())
			.map(|line| serde_json::from_str(line).map_err(Into::into))
			.collect()
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
		let events = Self::decode(&store.get(&path).await?)?;
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
		dates.xok()
	}

	/// Writes a daily archive and verifies its exact bytes by reading it back.
	pub async fn write(
		store: &BlobStore,
		date: &str,
		events: &[AnalyticsEvent],
	) -> Result<SmolPath> {
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

	/// The archive is lossless and its bytes are a pure function of the day: it
	/// is the primary copy once compaction deletes the segments, and a re-run
	/// must overwrite its own object rather than write a different one.
	#[beet_core::test]
	fn round_trips_a_day() {
		let events = events();
		let bytes = AnalyticsArchive::encode(&events).unwrap();
		// gzip, not the json it holds
		bytes[..2].to_vec().xpect_eq(vec![0x1f, 0x8b]);
		AnalyticsArchive::encode(&events)
			.unwrap()
			.xpect_eq(bytes.clone());
		// ..and the order events arrive in does not change the object
		let mut shuffled = events.clone();
		shuffled.reverse();
		AnalyticsArchive::encode(&shuffled)
			.unwrap()
			.xpect_eq(bytes.clone());

		let mut decoded = AnalyticsArchive::decode(&bytes).unwrap();
		decoded.sort_by_key(|event| event.id);
		let mut expected = events;
		expected.sort_by_key(|event| event.id);
		decoded.len().xpect_eq(3);
		decoded
			.iter()
			.map(|event| event.path.clone())
			.collect::<Vec<_>>()
			.xpect_eq(
				expected
					.iter()
					.map(|event| event.path.clone())
					.collect::<Vec<_>>(),
			);
		AnalyticsArchive::object_path("2026-08-01")
			.to_string()
			.xpect_eq("analytics/raw/2026-08-01.ndjson.gz");
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
		AnalyticsArchive::decode(&store.get(&path).await.unwrap())
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
