//! The cold copy of a day's raw events, and the codec both halves go through.
use crate::exports::bytes::Bytes;
use crate::prelude::*;
use beet_core::prelude::*;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::Read;
use std::io::Write;

/// The gzipped newline-delimited json a day of raw [`AnalyticsEvent`]s is
/// archived as, and where in a store it lands.
///
/// The first act of the retention pipeline, and the reason expiring raw events
/// is not information loss: the whole history is a few megabytes compressed, so
/// object storage of every event ever recorded costs effectively nothing while
/// the table it was scanned out of does not. Nothing stamps an expiry on a row
/// this has not already written.
///
/// Daily objects rather than monthly ones so an archive write can never race the
/// expiry window of a day it does not cover, and `ndjson` rather than one json
/// array so a reader can stream a day without holding it whole.
pub struct AnalyticsArchive;

impl AnalyticsArchive {
	/// The prefix analytics owns in the archive store. Other durable
	/// runtime-born data (table exports, generated reports) joins the store
	/// under its own prefix rather than this one.
	pub const PREFIX: &'static str = "analytics/raw";

	/// The object a day's events archive to, ie
	/// `analytics/raw/2026-08-01.ndjson.gz`. A pure function of the date, so a
	/// re-run overwrites its own object rather than accumulating copies.
	pub fn object_path(date: &str) -> SmolPath {
		SmolPath::new(format!("{}/{date}.ndjson.gz", Self::PREFIX))
	}

	/// Encode `events` as gzipped ndjson, ordered by row id so the same day
	/// always encodes to the same bytes.
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

	/// Decode an archive object back into its events, ie to re-derive an
	/// aggregate schema over a history the table no longer holds.
	pub fn decode(bytes: &[u8]) -> Result<Vec<AnalyticsEvent>> {
		let mut ndjson = String::new();
		GzDecoder::new(bytes).read_to_string(&mut ndjson)?;
		ndjson
			.lines()
			.filter(|line| !line.trim().is_empty())
			.map(|line| serde_json::from_str(line).map_err(Into::into))
			.collect()
	}

	/// Write `date`'s events into `store` and confirm the object landed,
	/// returning its path.
	///
	/// The read-back is the point: the expiry stamped later in the run is only
	/// safe because this object is known to exist, and a store that accepted a
	/// write it did not keep is exactly the failure that would make it not.
	pub async fn write(
		store: &BlobStore,
		date: &str,
		events: &[AnalyticsEvent],
	) -> Result<SmolPath> {
		let blob = store.blob(Self::object_path(date));
		blob.insert(Self::encode(events)?).await?;
		if !blob.exists().await? {
			bevybail!(
				"the analytics archive for {date} was written to {} but is not there: \
				 nothing may expire until it is",
				store.describe()
			);
		}
		blob.path().clone().xok()
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

	/// The archive is lossless and its bytes are a pure function of the day:
	/// the raws it holds are the ONLY copy once the table expires them, and a
	/// re-run must overwrite its own object rather than write a different one.
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

	/// An empty day still archives, so "the object exists" is an unambiguous
	/// answer to "has this day been archived".
	#[beet_core::test]
	async fn writes_and_confirms_the_object() {
		let store = BlobStore::temp();
		let path = AnalyticsArchive::write(&store, "2026-08-01", &events())
			.await
			.unwrap();
		path.to_string()
			.xpect_eq("analytics/raw/2026-08-01.ndjson.gz");
		AnalyticsArchive::decode(&store.get(&path).await.unwrap())
			.unwrap()
			.len()
			.xpect_eq(3);
		AnalyticsArchive::write(&store, "2026-08-02", &[])
			.await
			.unwrap();
		store
			.blob(AnalyticsArchive::object_path("2026-08-02"))
			.exists()
			.await
			.unwrap()
			.xpect_true();
	}
}
