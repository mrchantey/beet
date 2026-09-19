//! Reading the cold copy back, which is the only thing that makes it one.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

impl MailColdProbe {
	/// The instant a snapshot key names, ie `sqlite/2026/09/14/143522Z.db`
	/// is 14:35:22 UTC on that day. The key is the box's own clock at the
	/// moment it snapshotted, which is the age that matters: an object's
	/// last-modified in the cold bucket says when it was COPIED, and a nightly
	/// copy of a stale snapshot would look fresh by that measure.
	pub fn snapshot_time(key: &str) -> Option<Timestamp> {
		let rest = key.strip_prefix(StalwartBlock::BACKUP_PREFIX)?;
		let mut parts = rest.trim_start_matches('/').split('/');
		let (year, month, day, file) =
			(parts.next()?, parts.next()?, parts.next()?, parts.next()?);
		let midnight = Timestamp::parse_date(&format!("{year}-{month}-{day}"))?;
		let clock = file.strip_suffix("Z.db")?;
		if clock.len() != 6 {
			return None;
		}
		let (hours, minutes, seconds) = (
			clock[0..2].parse::<i64>().ok()?,
			clock[2..4].parse::<i64>().ok()?,
			clock[4..6].parse::<i64>().ok()?,
		);
		Timestamp::from_secs(
			midnight.secs() + hours * 3600 + minutes * 60 + seconds,
		)
		.xmap(Some)
	}

	/// The live key a cold blob was copied from: the cold bucket mirrors the
	/// blob bucket under one prefix.
	pub fn live_blob_key(cold_key: &str) -> Option<&str> {
		cold_key
			.strip_prefix(StalwartBlock::COLD_BLOBS_PREFIX)?
			.strip_prefix('/')
	}

	/// Whether an export is this stack's: its `origin` names the app and
	/// stage, so a document copied from another stage (a drill's, another
	/// account's) cannot pass as the one a restore would read.
	pub fn check_origin(
		document: &SecretsDocument,
		stack: &ResolvedStack,
	) -> Result {
		let Some(origin) = &document.origin else {
			bevybail!(
				"the export carries no origin, so it is not a stack export"
			);
		};
		if origin.app != *stack.app_name() || origin.stage != *stack.stage() {
			bevybail!(
				"the export came from `{}--{}`, not this stack `{}--{}`",
				origin.app,
				origin.stage,
				stack.app_name(),
				stack.stage()
			);
		}
		Ok(())
	}
}

/// The `<Secrets>` declaration targeting the stack's cold bucket
/// (`{StoreRef($cold_backups)}`), which the dated exports are written
/// beside: its path names the series dir and format.
#[derive(SystemParam)]
struct ColdExportQuery<'w, 's> {
	declared: Query<'w, 's, (&'static Secrets, &'static StoreRef)>,
	buckets: Query<'w, 's, &'static R2BucketBlock>,
}

impl ColdExportQuery<'_, '_> {
	fn find(&self, cold: &R2BucketBlock) -> Option<Secrets> {
		self.declared
			.iter()
			.find(|(_, store_ref)| {
				self.buckets
					.get(store_ref.store())
					.is_ok_and(|bucket| bucket.label() == cold.label())
			})
			.map(|(secrets, _)| secrets.clone())
	}
}

/// Lists the cold bucket and reads a sample of each prefix back against the
/// live side, failing on anything missing, stale or different.
/// `<MailColdProbe/>` — assert the cold copy is fresh and byte-for-byte what
/// the live side holds.
///
/// Three prefixes, three questions. The newest snapshot under `sqlite/` must
/// have been TAKEN within `max_age` (by the clock in its key, not by when it
/// landed) and must hash to the same bytes as the live archive's copy. One
/// blob under `blobs/` must hash to the same bytes as the live blob bucket's
/// object at the same key, and a live bucket with blobs beside a cold prefix
/// with none is the copy having never run. And an export must exist under
/// `secrets/`, since a restore into a fresh account starts by reading it.
///
/// Read-only against both sides, cheap (three small downloads), and made from
/// the deploy machine with the same parked token the box copies with, so it
/// proves what a restore would find rather than what the box believes it
/// wrote. It is the tail of every deploy, after [`MailColdPush`], and a verb
/// of its own for any other day.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailColdProbe(
	/// How old the newest cold snapshot may be. A day and a half: strictly
	/// more than one nightly period with its jitter, strictly less than two,
	/// so one missed night fails.
	#[field(default = Duration::from_secs(36 * 3600))]
	max_age: Duration,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let cold = ColdStore::resolve(mail.cold_store()?, &mail.secrets).await?;
	let region = mail.stack.region().to_string();
	let archive = LiveStore {
		region: region.clone(),
		bucket: mail
			.stack
			.resource_name(mail.mail_box.backup_bucket().clone()),
	};
	let blobs = LiveStore {
		region,
		bucket: mail
			.stack
			.resource_name(mail.mail_box.blob_bucket().clone()),
	};
	let work_dir = mail.project.work_dir().clone();

	// the snapshot: fresh by its own clock, identical to the live copy
	let prefix = format!("{}/", StalwartBlock::BACKUP_PREFIX);
	let listing = cold.list(&prefix).await?;
	let Some(newest) = listing.newest(".db") else {
		bevybail!(
			"s3://{}/{prefix} holds no snapshot, so the cold copy has never \
			run: `systemctl start {}` on the box, or `MailColdPush`",
			cold.bucket,
			StalwartBlock::COLD_UNIT
		);
	};
	let taken = MailColdProbe::snapshot_time(&newest.key).ok_or_else(|| {
		bevyhow!(
			"cold snapshot key {} is not in the box's YYYY/MM/DD/HHMMSSZ shape, \
			so its age cannot be read",
			newest.key
		)
	})?;
	let age = Timestamp::now().secs().saturating_sub(taken.secs());
	if age > max_age.as_secs() as i64 {
		bevybail!(
			"the newest cold snapshot {} was taken {} ago, which is past the \
			{} allowed: either the box has stopped snapshotting or the cold \
			copy has stopped copying, and `systemctl list-timers` on the box \
			says which",
			newest.key,
			time_ext::pretty_print_duration(Duration::from_secs(age as u64)),
			time_ext::pretty_print_duration(max_age),
		);
	}
	compare(
		&cold,
		&archive,
		&newest.key,
		&newest.key,
		&work_dir,
		"snapshot",
	)
	.await?;
	info!(
		"cold snapshot {} ({} bytes, taken {} ago) matches the live archive \
		byte for byte",
		newest.key,
		newest.size,
		time_ext::pretty_print_duration(Duration::from_secs(age as u64))
	);

	// one blob, byte for byte; none at all is the copy having never reached
	// the blobs, unless the live store is empty too
	let blob_prefix = format!("{}/", StalwartBlock::COLD_BLOBS_PREFIX);
	let cold_blobs = cold.list(&blob_prefix).await?;
	match cold_blobs.newest("") {
		Some(sample) => {
			let live_key = MailColdProbe::live_blob_key(&sample.key)
				.ok_or_else(|| {
					bevyhow!(
						"cold blob {} is outside {blob_prefix}",
						sample.key
					)
				})?;
			compare(&cold, &blobs, &sample.key, live_key, &work_dir, "blob")
				.await?;
			info!(
				"cold blob {} ({} bytes) matches the live blob store byte for \
				byte, {} blob(s) listed on the first page",
				sample.key,
				sample.size,
				cold_blobs.0.len()
			);
		}
		None => {
			let live = blobs.list("").await?;
			if !live.0.is_empty() {
				bevybail!(
					"s3://{}/{blob_prefix} holds no blob while the live store \
					holds {}: the cold copy has never reached the message \
					bodies",
					cold.bucket,
					live.0.len()
				);
			}
			info!("no blobs on either side yet, which is a new mail store");
		}
	}

	// the export: the newest of the dated series is this stack's and opens
	// with this identity, since a restore into a fresh account starts there
	let cold_block = mail.cold_store()?.clone();
	let Some(declared) = cx
		.caller
		.with_state::<ColdExportQuery, _>(move |_, query| {
			query.find(&cold_block)
		})
		.await?
	else {
		bevybail!(
			"no `<Secrets>` targets the cold bucket, so no export is written \
			into it: declare `<Secrets label=\"mail-cold\" path=\"secrets/export.toml\" \
			{{StoreRef($cold_backups)}}/>` and `<SecretsExport document={{$mail_cold}} \
			dated=true/>`"
		);
	};
	let handle = SecretsHandle::new(BlobStore::temp(), declared.path.as_str())?;
	let extension = declared.media_type()?.extension().unwrap_or_default();
	let series = declared
		.path
		.as_str()
		.rsplit_once('/')
		.map(|(dir, _)| format!("{dir}/"))
		.unwrap_or_default();
	let exports = cold.list(&series).await?;
	let Some(export) = exports
		.0
		.iter()
		.filter(|object| {
			object
				.key
				.strip_prefix(&series)
				.is_some_and(|key| SecretsHandle::is_dated(key, extension))
		})
		.max_by(|left, right| left.key.cmp(&right.key))
	else {
		bevybail!(
			"s3://{}/{series} holds no export, so a restore into a fresh \
			account would have no DKIM key to sign with: `deploy` or \
			`provision` writes one through `<SecretsExport dated=true/>`",
			cold.bucket
		);
	};
	let local = work_dir.join("cold-probe-export.toml");
	let checked = async {
		cold.download(&export.key, &local).await?;
		let document = SecretsDocument::parse(
			handle.media_type()?,
			&fs_ext::read(&local)?,
		)?;
		MailColdProbe::check_origin(&document, &mail.stack)?;
		let opened = document.open(&AgeIdentityFile::require()?)?;
		if opened.secrets.is_empty() {
			bevybail!(
				"this identity opens no group of the export (locked: {}): a \
				restore from it would find nothing",
				opened.locked.join(", ")
			);
		}
		opened.secrets.len().xok()
	}
	.await;
	fs_ext::remove(&local).ok();
	let count = checked?;
	info!(
		"newest secrets export {} ({} bytes, uploaded {}) is this stack's and \
		opens {count} record(s)",
		export.key, export.size, export.last_modified
	);
	Pass(cx.input).xok()
}

/// Download `cold_key` from the cold side and `live_key` from the live side
/// and compare their digests, removing both files whatever the outcome.
async fn compare(
	cold: &ColdStore,
	live: &LiveStore,
	cold_key: &str,
	live_key: &str,
	work_dir: &AbsPath,
	what: &str,
) -> Result {
	let cold_path = work_dir.join(format!("cold-probe-{what}.cold"));
	let live_path = work_dir.join(format!("cold-probe-{what}.live"));
	let result = async {
		cold.download(cold_key, &cold_path).await?;
		live.download(live_key, &live_path).await?;
		let cold_digest = ListedObject::digest(&cold_path)?;
		let live_digest = ListedObject::digest(&live_path)?;
		if cold_digest != live_digest {
			bevybail!(
				"cold {what} {cold_key} differs from live {live_key}: sha256 \
				{cold_digest} against {live_digest}. The cold side holds \
				something other than what it was meant to copy"
			);
		}
		Ok(())
	}
	.await;
	fs_ext::remove(&cold_path).ok();
	fs_ext::remove(&live_path).ok();
	result
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The age is read off the key the box wrote, which is the snapshot's
	/// own clock: a last-modified in the cold bucket would date the copy, and
	/// a nightly copy of a stale snapshot would pass by it.
	#[beet_core::test]
	fn snapshot_age_is_read_off_the_key() {
		let taken =
			MailColdProbe::snapshot_time("sqlite/2026/09/14/143522Z.db")
				.unwrap();
		taken.format_iso8601().xpect_eq("2026-09-14T14:35:22.000Z");
		MailColdProbe::snapshot_time("sqlite/notes.db").xpect_none();
		MailColdProbe::snapshot_time("other/2026/09/14/143522Z.db")
			.xpect_none();
	}

	/// A cold blob's live key is the cold key without the mirror prefix,
	/// since the copy mirrors the whole blob bucket under one directory.
	#[beet_core::test]
	fn cold_blobs_map_back_to_live_keys() {
		MailColdProbe::live_blob_key("blobs/ab/cdef")
			.unwrap()
			.xpect_eq("ab/cdef");
		MailColdProbe::live_blob_key("sqlite/x.db").xpect_none();
	}

	/// An export from another stage, or with no origin, is not this stack's.
	#[beet_core::test]
	fn the_export_must_name_this_stack() {
		let stack = Stack::new("beetmash-mail")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		let mut document = SecretsDocument::default();
		MailColdProbe::check_origin(&document, &stack)
			.unwrap_err()
			.to_string()
			.xpect_contains("no origin");
		let origin = SecretsOrigin {
			app: "beetmash-mail".into(),
			stage: "drill".into(),
			region: None,
			provider: "ssm".into(),
			exported: Timestamp::UNIX_EPOCH,
		};
		document.origin = Some(origin.clone());
		MailColdProbe::check_origin(&document, &stack)
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"`beetmash-mail--drill`, not this stack `beetmash-mail--prod`",
			);
		document.origin = Some(SecretsOrigin {
			stage: "prod".into(),
			..origin
		});
		MailColdProbe::check_origin(&document, &stack).unwrap();
	}
}
