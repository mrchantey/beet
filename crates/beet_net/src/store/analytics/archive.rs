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

	/// Returns the object path for one UTC date, under the codec every object
	/// is written under. A day archived under another codec is read from
	/// [`Self::existing_path`] and rewritten here.
	pub fn object_path(date: &str) -> RelPath {
		Self::codec_path(date, JsonlCodec::default().extension())
	}

	fn codec_path(date: &str, extension: &str) -> RelPath {
		RelPath::new(format!("{}/{date}.{extension}", Self::PREFIX))
	}

	/// Returns the UTC date encoded in a daily archive path, under any JSONL
	/// codec, compiled in or not.
	pub(crate) fn date(path: &RelPath) -> Option<SmolStr> {
		let (date, _) = path
			.as_str()
			.strip_prefix(Self::PREFIX)?
			.strip_prefix('/')
			.and_then(JsonlCodec::split_path)?;
		(!date.contains('/') && Timestamp::parse_date(date).is_some())
			.then(|| date.into())
	}

	/// Encodes events as deterministic JSONL ordered by event ID, at the
	/// archive-grade level: written once a night, kept for good.
	pub fn encode(events: &[AnalyticsEvent]) -> Result<Bytes> {
		Jsonl::encode(
			JsonlCodec::default(),
			JsonlLevel::Best,
			AnalyticsEvent::by_id(events),
		)
	}

	/// The path a date is archived under, if any: the written codec's first,
	/// then any other the store still holds.
	pub async fn existing_path(
		store: &BlobStore,
		date: &str,
	) -> Result<Option<RelPath>> {
		for extension in JsonlCodec::EXTENSIONS {
			let path = Self::codec_path(date, extension);
			if store.exists(&path).await? {
				return Some(path).xok();
			}
		}
		None.xok()
	}

	/// Reads and validates a daily archive when it exists.
	pub(crate) async fn read(
		store: &BlobStore,
		date: &str,
	) -> Result<Option<Vec<AnalyticsEvent>>> {
		let Some(path) = Self::existing_path(store, date).await? else {
			return None.xok();
		};
		let events = Jsonl::decode_path::<AnalyticsEvent>(
			&path,
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

	/// Writes a daily archive, verifies its exact bytes by reading it back, and
	/// only then removes the day's archive under any other codec, so a day
	/// never has two.
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
		for extension in JsonlCodec::EXTENSIONS {
			let stale = Self::codec_path(date, extension);
			if stale != path && store.exists(&stale).await? {
				store.remove(&stale).await?;
			}
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
		// a gzip archive is still a day, whether or not this build reads it
		AnalyticsArchive::date(&RelPath::new(
			"analytics/raw/2026-08-01.jsonl.gz",
		))
		.xpect_eq(Some("2026-08-01".into()));
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

	/// A day archived under gzip is read as-is and, once rewritten, exists
	/// only under the written codec.
	#[cfg(feature = "gzip")]
	#[beet_core::test]
	async fn rewrites_a_gzip_archive_as_zstd() {
		let store = BlobStore::temp();
		let events = events();
		let date = events[0].date();
		let gzip = RelPath::new(format!(
			"{}/{date}.jsonl.gz",
			AnalyticsArchive::PREFIX
		));
		store
			.insert(
				&gzip,
				Jsonl::encode(JsonlCodec::Gzip, JsonlLevel::Fast, &events)
					.unwrap(),
			)
			.await
			.unwrap();
		AnalyticsArchive::existing_path(&store, &date)
			.await
			.unwrap()
			.xpect_eq(Some(gzip.clone()));
		AnalyticsArchive::read(&store, &date)
			.await
			.unwrap()
			.unwrap()
			.len()
			.xpect_eq(3);
		AnalyticsArchive::dates(&store)
			.await
			.unwrap()
			.xpect_eq(vec![date.clone()]);

		let zstd = AnalyticsArchive::write(&store, &date, &events)
			.await
			.unwrap();
		zstd.xpect_eq(AnalyticsArchive::object_path(&date));
		store.exists(&gzip).await.unwrap().xpect_false();
		AnalyticsArchive::existing_path(&store, &date)
			.await
			.unwrap()
			.xpect_eq(Some(zstd));
	}
}
