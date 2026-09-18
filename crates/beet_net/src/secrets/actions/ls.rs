//! `secrets/ls`: a document's index, never a value.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// List a document's groups and records: every record's name, role, group,
/// note and modified time, with a lock on each group this identity cannot
/// open. The index is plaintext, so no identity is needed and no value is
/// ever printed.
///
/// ```sh
/// beet secrets/ls                      # the declared document
/// beet secrets/ls --vault=mail-prod    # a declared label
/// beet secrets/ls --vault=~/personal.toml.age
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("ls"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsLs(cx: ActionContext<Request>) -> Result<Response> {
	let vault = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let document = vault.read_document().await?;
	// no identity is fine: every group is then locked
	let identities = AgeIdentityFile::discover()?.unwrap_or_default();
	let opened = document.open(&identities)?;
	let mut out = format!(
		"{}: {} group(s), {} record(s)\n",
		vault.describe(),
		document.groups.len(),
		document.secrets.len()
	);
	if let Some(origin) = &document.origin {
		writeln!(
			out,
			"origin: {}/{} from {} at {}",
			origin.app,
			origin.stage,
			origin.provider,
			origin.exported.format_iso8601()
		)?;
	}
	let groups = document
		.groups
		.iter()
		.map(|(name, group)| {
			vec![
				name.to_string(),
				format!("{} recipient(s)", group.recipients.len()),
				group_status(name, &opened).to_string(),
			]
		})
		.collect::<Vec<_>>();
	write_table(&mut out, "groups:", &groups)?;
	let records = document
		.secrets
		.iter()
		.map(|(name, record)| {
			let group = record.group();
			vec![
				name.to_string(),
				record.role.map(|role| role.to_string()).unwrap_or_default(),
				match opened.can_open(group) {
					true => group.to_string(),
					false => format!("{group} {LOCK}"),
				},
				record
					.modified
					.map(|modified| modified.format_iso8601())
					.unwrap_or_default(),
				record.note.as_deref().unwrap_or_default().to_string(),
			]
		})
		.collect::<Vec<_>>();
	write_table(&mut out, "records:", &records)?;
	Response::ok_text(out).xok()
}

/// The lock a group this identity cannot open is marked with.
pub(crate) const LOCK: &str = "🔒";

/// How a group fared for this identity, for the listing.
fn group_status(name: &str, opened: &OpenSecrets) -> String {
	if opened.drifted.iter().any(|group| group == name) {
		return "opens, list changed since sealed: run `secrets/rekey`".into();
	}
	if opened.can_open(name) {
		return "opens".into();
	}
	if opened.pending.iter().any(|group| group == name) {
		return format!(
			"{LOCK} listed but sealed before: a member runs `secrets/rekey`"
		);
	}
	format!("{LOCK} not a member")
}

/// Append `rows` under `heading` with every column padded to its widest
/// cell, nothing when there are no rows.
fn write_table(
	out: &mut String,
	heading: &str,
	rows: &[Vec<String>],
) -> Result {
	writeln!(out, "{heading}")?;
	let Some(columns) = rows.first().map(Vec::len) else {
		writeln!(out, "  (none)")?;
		return OK;
	};
	let widths = (0..columns)
		.map(|column| {
			rows.iter()
				.map(|row| width(&row[column]))
				.max()
				.unwrap_or(0)
		})
		.collect::<Vec<_>>();
	for row in rows {
		let line = row
			.iter()
			.zip(&widths)
			.map(|(cell, column)| {
				format!("{cell}{}", " ".repeat(column - width(cell)))
			})
			.collect::<Vec<_>>()
			.join("  ");
		writeln!(out, "  {}", line.trim_end())?;
	}
	OK
}

/// A cell's width in terminal columns: the lock renders two wide.
fn width(cell: &str) -> usize {
	cell.chars().count() + cell.matches(LOCK).count()
}

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A record with a fixed `modified`, so the listing snapshots.
	fn record(role: Option<SecretRole>, note: &str) -> SecretRecord {
		SecretRecord {
			role,
			note: Some(note.into()),
			modified: Some(Timestamp::parse_date("2026-09-18").unwrap()),
			..default()
		}
	}

	/// The index lists every record with its group's lock state and no
	/// value, and a stranger's group is locked.
	#[beet_core::test]
	async fn lists_the_index_without_values() {
		let fixture = VerbWorld::new();
		let stranger = AgeIdentity::generate();
		let mut document = fixture.document().await;
		document.groups.insert(
			"agents".into(),
			SecretsGroup::new(vec![stranger.to_recipient()]),
		);
		let mut theirs = AgeIdentityFile::default();
		theirs.push(stranger);
		document
			.set(
				&theirs,
				"CF_API_TOKEN",
				"cf-PRIVATE",
				record(Some(SecretRole::EnvVar), "dns and workers")
					.with_group("agents"),
			)
			.unwrap();
		document
			.set(
				&fixture.identities(),
				"OPENAI_API_KEY",
				"sk-PRIVATE",
				record(Some(SecretRole::EnvVar), "billing account"),
			)
			.unwrap();
		document
			.set(
				&fixture.identities(),
				"dkim-example-com",
				"key-PRIVATE",
				record(None, "the signing key"),
			)
			.unwrap();
		fixture
			.vault("secrets.toml.age")
			.write_document(&document)
			.await
			.unwrap();
		let mut fixture = fixture;
		fixture
			.call_str(SecretsLs, Request::get("/"))
			.await
			.unwrap()
			.xnot()
			.xpect_contains("PRIVATE")
			.xpect_snapshot();
	}

	/// The missing conventional file says how it is written.
	#[beet_core::test]
	async fn names_a_missing_document() {
		let mut fixture = VerbWorld::new();
		fixture
			.call(SecretsLs, Request::get("/"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("secrets.toml.age");
	}
}
