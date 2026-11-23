//! The one-time import of analytics recorded before events were rows.
use crate::prelude::*;
use beet_core::prelude::*;
use std::io::Read;

/// Imports the analytics history a store kept as zstd JSONL objects, before
/// events were rows: every daily archive (`analytics/raw/<date>.jsonl.zst`)
/// and every unrolled segment (`analytics/raw/segments/**.jsonl.zst`) under
/// the stores it reads, decoded into `analytics_event` rows.
///
/// Each event's key is a pure function of its id and timestamp (its TID's
/// sub-millisecond digits and clock id drawn from the id), and a page view's
/// heartbeats are reduced to its last first, so a rerun rewrites the same
/// rows and an archive overlapping a segment imports each event once. The old
/// objects are never touched: removing them is the operator's, once the
/// import is checked.
pub struct AnalyticsImport {
	sources: Vec<BlobStore>,
	to: AnalyticsStore,
}

/// What an [`AnalyticsImport`] read and wrote.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnalyticsImportReport {
	/// Legacy objects decoded.
	pub objects: usize,
	/// Events read, heartbeats included.
	pub read: usize,
	/// Rows written, one per event once deduplicated.
	pub written: usize,
}

impl core::fmt::Display for AnalyticsImportReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"analytics import: {} events from {} objects, {} rows written",
			self.read, self.objects, self.written
		)
	}
}

impl AnalyticsImport {
	/// The prefix every legacy raw object lives under.
	const PREFIX: &'static str = "analytics/raw";
	/// The extension every legacy raw object carries.
	const EXTENSION: &'static str = ".jsonl.zst";

	/// Import the legacy objects in `sources` into `to`.
	pub fn new(sources: Vec<BlobStore>, to: AnalyticsStore) -> Self {
		Self { sources, to }
	}

	/// Decode every legacy object and write its events as rows.
	pub async fn run(&self) -> Result<AnalyticsImportReport> {
		let mut report = AnalyticsImportReport::default();
		let mut events = Vec::new();
		for source in &self.sources {
			for path in Self::objects(source).await? {
				let decoded =
					Self::decode(&source.get(&path).await?).map_err(|err| {
						bevyhow!(
							"legacy analytics `{path}` in {}: {err}",
							source.describe()
						)
					})?;
				report.objects += 1;
				report.read += decoded.len();
				events.extend(decoded);
			}
		}
		let rows = AnalyticsEvent::dedupe(events)
			.into_iter()
			.map(Self::keyed)
			.collect::<Vec<_>>();
		report.written = rows.len();
		let table = self.to.events();
		rows.into_iter()
			.map(|event| table.put(event))
			.xmap(|puts| {
				async_ext::try_join_all_bounded(
					BlobStore::GET_ALL_CONCURRENCY,
					puts,
				)
			})
			.await?;
		report.xok()
	}

	/// Every legacy object in `store`, sorted.
	async fn objects(store: &BlobStore) -> Result<Vec<RelPath>> {
		if !store.store_exists().await? {
			return Vec::new().xok();
		}
		let mut paths = store
			.list()
			.await?
			.into_iter()
			.filter(|path| {
				path.as_str().starts_with(Self::PREFIX)
					&& path.as_str().ends_with(Self::EXTENSION)
			})
			.collect::<Vec<_>>();
		paths.sort();
		paths.xok()
	}

	/// The events of one zstd JSONL object, one json event a line.
	pub fn decode(bytes: &[u8]) -> Result<Vec<AnalyticsEvent>> {
		let mut text = String::new();
		ruzstd::decoding::StreamingDecoder::new(bytes)
			.map_err(|err| bevyhow!("not a zstd frame: {err}"))?
			.read_to_string(&mut text)?;
		text.lines()
			.filter(|line| !line.trim().is_empty())
			.map(|line| {
				serde_json::from_str::<AnalyticsEvent>(line)
					.map_err(BevyError::from)
			})
			.collect()
	}

	/// The event under the key its id and timestamp determine.
	fn keyed(mut event: AnalyticsEvent) -> AnalyticsEvent {
		let id = event.id.as_u128();
		event.key = Tid::from_parts(
			event.timestamp * 1000 + (id % 1000) as u64,
			((id / 1000) % 1024) as u16,
		);
		event
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A fixture object, compressed as the legacy writer did.
	fn object(events: &[AnalyticsEvent]) -> bytes::Bytes {
		let lines = events
			.iter()
			.map(|event| serde_json::to_string(event).unwrap())
			.collect::<Vec<_>>()
			.join("\n");
		ruzstd::encoding::compress_to_vec(
			lines.as_bytes(),
			ruzstd::encoding::CompressionLevel::Fastest,
		)
		.into()
	}

	fn view(path: &str, days: u64) -> AnalyticsEvent {
		let mut event =
			AnalyticsEvent::new(path, AnalyticsEventData::PageView {
				duration_ms: 1_000,
				referrer: None,
				title: None,
				client: default(),
			});
		event.timestamp -= days * 86_400_000;
		event
	}

	/// A legacy event as the old writer wrote it, with no key.
	fn legacy_json(event: &AnalyticsEvent) -> serde_json::Value {
		let mut json = serde_json::to_value(event).unwrap();
		json.as_object_mut().unwrap().remove("key");
		json
	}

	/// Archives and segments import once each event, heartbeats reduced to
	/// the last, and a rerun rewrites the same rows.
	#[beet_core::test]
	async fn imports_archives_and_segments() {
		let archive = BlobStore::temp();
		let analytics = BlobStore::temp();
		let old = view("/old", 3);
		let mut heartbeat = old.clone();
		heartbeat.timestamp += 5_000;
		heartbeat.path = "/old-later".into();
		let recent = view("/recent", 1);
		// keyless json, as the legacy writer wrote it
		let legacy = |events: &[AnalyticsEvent]| {
			let lines = events
				.iter()
				.map(|event| legacy_json(event).to_string())
				.collect::<Vec<_>>()
				.join("\n");
			bytes::Bytes::from(ruzstd::encoding::compress_to_vec(
				lines.as_bytes(),
				ruzstd::encoding::CompressionLevel::Fastest,
			))
		};
		archive
			.insert(
				&RelPath::new(format!(
					"analytics/raw/{}.jsonl.zst",
					old.date()
				)),
				legacy(&[old.clone()]),
			)
			.await
			.unwrap();
		analytics
			.insert(
				&RelPath::new(format!(
					"analytics/raw/segments/{}/w/1-1.jsonl.zst",
					old.date()
				)),
				legacy(&[heartbeat]),
			)
			.await
			.unwrap();
		analytics
			.insert(
				&RelPath::new(format!(
					"analytics/raw/segments/{}/w/2-1.jsonl.zst",
					recent.date()
				)),
				object(&[recent.clone()]),
			)
			.await
			.unwrap();
		// another writer's object is not read
		analytics
			.insert(&RelPath::new("analytics/other.jsonl"), "{}")
			.await
			.unwrap();
		let to = AnalyticsStore::new(TableStore::temp());
		let import = AnalyticsImport::new(vec![archive, analytics], to.clone());
		let report = import.run().await.unwrap();
		report.objects.xpect_eq(3);
		report.read.xpect_eq(3);
		report.written.xpect_eq(2);
		let date = Date::parse(&old.date()).unwrap();
		let day = to.day(date).await.unwrap();
		day.len().xpect_eq(1);
		day[0].path.as_str().xpect_eq("/old-later");
		// idempotent
		import.run().await.unwrap();
		to.events().list().await.unwrap().len().xpect_eq(2);
		// the rollup then reads the imported history
		AnalyticsRollupRun::new(to.clone(), None)
			.call()
			.await
			.unwrap();
		to.summary(false).await.unwrap().page_views.xpect_eq(2);
	}
}
