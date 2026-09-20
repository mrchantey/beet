//! `secrets/ls`: a document's index, never a value.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// List a document's index, one block per group: the group's recipient
/// count and whether this identity opens it, then every record as its name,
/// role and modified time on one line with its note and rotation indented
/// under it. The index is plaintext, so no identity is needed and no value
/// is ever printed.
///
/// ```sh
/// beet secrets/ls                         # the declared document
/// beet secrets/ls --document=mail-prod    # a declared label
/// beet secrets/ls --document=~/personal.toml
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("ls"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsLs(cx: ActionContext<Request>) -> Result<Response> {
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let document = handle.read().await?;
	// no identity is fine: every group is then locked
	let identities = AgeIdentityFile::discover()?.unwrap_or_default();
	let opened = document.open(&identities)?;
	let mut out = format!(
		"{}: {} group(s), {} record(s)\n",
		handle.describe(),
		document.groups.len(),
		document.record_count()
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
	for (name, group) in &document.groups {
		writeln!(
			out,
			"group `{name}` ({} recipient(s)): {}",
			group.recipients.len(),
			group_status(name, &opened)
		)?;
		let rows = group
			.secrets
			.iter()
			.map(|(name, record)| {
				(
					vec![
						name.to_string(),
						record
							.role
							.map(|role| role.to_string())
							.unwrap_or_default(),
						record
							.modified
							.map(|modified| modified.format_iso8601())
							.unwrap_or_default(),
					],
					[
						record.note.as_deref().map(str::to_string),
						record.rotation.as_ref().map(ToString::to_string),
					],
				)
			})
			.collect::<Vec<_>>();
		write_records(&mut out, &rows)?;
	}
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

/// Append every record: its columns padded to the widest cell, then each
/// of its detail lines indented under it; `(none)` when there are no rows.
fn write_records(
	out: &mut String,
	rows: &[(Vec<String>, [Option<String>; 2])],
) -> Result {
	let Some(columns) = rows.first().map(|(cells, _)| cells.len()) else {
		writeln!(out, "  (none)")?;
		return OK;
	};
	let widths = (0..columns)
		.map(|column| {
			rows.iter()
				.map(|(cells, _)| width(&cells[column]))
				.max()
				.unwrap_or(0)
		})
		.collect::<Vec<_>>();
	for (cells, details) in rows {
		let line = cells
			.iter()
			.zip(&widths)
			.map(|(cell, column)| {
				format!("{cell}{}", " ".repeat(column - width(cell)))
			})
			.collect::<Vec<_>>()
			.join("  ");
		writeln!(out, "  {}", line.trim_end())?;
		for detail in details.iter().flatten() {
			writeln!(out, "      {detail}")?;
		}
	}
	OK
}

/// A cell's width in terminal columns: the lock renders two wide.
fn width(cell: &str) -> usize {
	cell.chars().count() + cell.matches(LOCK).count()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
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
				"agents",
				"CF_API_TOKEN",
				"cf-PRIVATE",
				record(Some(SecretRole::EnvVar), "dns and workers"),
			)
			.unwrap();
		document
			.set(
				&fixture.identities(),
				"default",
				"OPENAI_API_KEY",
				"sk-PRIVATE",
				SecretRecord {
					rotation: Some(SecretRotation::manual(
						"platform.openai.com/api-keys",
					)),
					..record(Some(SecretRole::EnvVar), "billing account")
				},
			)
			.unwrap();
		document
			.set(
				&fixture.identities(),
				"default",
				"dkim-example-com",
				"key-PRIVATE",
				record(None, "the signing key"),
			)
			.unwrap();
		fixture
			.secrets("secrets.toml")
			.write(&document)
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
			.xpect_contains("secrets.toml");
	}
}
