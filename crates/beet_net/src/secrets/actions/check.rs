//! `secrets/check`: the identity and every document, verified.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Verify the secrets setup: the identity resolves; every declared document
/// (or the one `--document` names) reads and every group this identity
/// opens verifies against its index; every group's sealed recipient list
/// matches its list (else "run `secrets/rekey`"); and which groups this
/// identity is not in. A declared path that is unwritten but has a dated
/// series beside it (an export with `dated=true`) checks the newest of the
/// series. One line per item with a tick or the reason; the whole ledger
/// prints, then a non-zero exit on any failure.
///
/// ```sh
/// beet secrets/check
/// beet secrets/check --document=mail-prod
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("check"),
	ParamsPartial = ParamsPartial::new::<DocumentParams>()
)]
pub async fn SecretsCheck(cx: ActionContext<Request>) -> Result<Response> {
	let mut report = Report::default();
	let identities = match AgeIdentityFile::require() {
		Ok(identities) if !identities.is_empty() => {
			report.pass(format!(
				"identity: {} identities, recipients {}",
				identities.len(),
				identities
					.recipients()
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join(", ")
			));
			Some(identities)
		}
		Ok(_) => {
			report.fail("identity: the identity file holds no identity");
			None
		}
		Err(err) => {
			report.fail(format!("identity: {err}"));
			None
		}
	};
	let selector = cx.input.parse_params::<DocumentParams>()?.document;
	// the entry's own store, to tell a document declared in another one
	let entry_store = SecretsHandle::resolve(&cx.caller, None)
		.await
		.ok()
		.map(|handle| handle.store.root_key());
	let mut handles = match &selector {
		Some(_) => Vec::new(),
		None => SecretsHandle::declared(&cx.caller).await?,
	};
	// named, or nothing declared: the one document the selector resolves,
	// which undeclared and unwritten is nothing to check
	if handles.is_empty() {
		let handle =
			SecretsHandle::resolve(&cx.caller, selector.as_deref()).await;
		match (&selector, &handle) {
			(None, Ok(handle)) if !handle.exists().await? => report.note(
				"no `<Secrets>` is declared in this entry and no `secrets.toml` \
				is written beside it",
			),
			_ => handles.push((
				SmolStr::new(
					selector.as_deref().unwrap_or(Secrets::DEFAULT_LABEL),
				),
				handle,
			)),
		}
	}
	for (label, handle) in handles {
		match handle {
			Ok(handle) => {
				let elsewhere = entry_store
					.as_ref()
					.is_some_and(|entry| *entry != handle.store.root_key());
				report
					.document(&handle, identities.as_ref(), elsewhere)
					.await?
			}
			Err(err) => report.fail(format!("document `{label}`: {err}")),
		}
	}
	report.into_response()
}

/// The lines of the report and whether any failed.
#[derive(Default)]
struct Report {
	lines: String,
	failed: bool,
}

impl Report {
	fn pass(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "✓ {}", line.as_ref()).ok();
	}

	fn note(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "- {}", line.as_ref()).ok();
	}

	fn fail(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "✗ {}", line.as_ref()).ok();
		self.failed = true;
	}

	/// The lines for one document: whether it exists and reads, and every
	/// group's state for this identity. One declared in another store than
	/// the entry's (`elsewhere`, ie an export target in a bucket) and not
	/// found there is a note rather than a failure: this launch reaches
	/// that store under its own credentials, while the verb that writes it
	/// resolves the bucket's parked ones (`cold-probe` reads it back).
	async fn document(
		&mut self,
		handle: &SecretsHandle,
		identities: Option<&AgeIdentityFile>,
		elsewhere: bool,
	) -> Result<()> {
		let name = handle.describe();
		// a dated series stands in for its unwritten declared path
		let newest = match handle.exists().await? {
			true => None,
			false => handle.newest_dated().await?,
		};
		let handle = match &newest {
			Some(newest) => {
				self.note(format!(
					"document {name}: a dated series, checking the newest ({})",
					newest.path
				));
				newest
			}
			None if !handle.exists().await? && elsewhere => {
				return self
					.note(format!(
						"document {name}: not found in its store as this launch \
						reaches it; the verb that reads it back verifies it \
						(`cold-probe` for a cold bucket's export)"
					))
					.xok();
			}
			None if !handle.exists().await? => {
				return self
					.fail(format!(
						"document {name}: not written yet (`secrets/set` writes a \
						document, `<SecretsExport>` an export)"
					))
					.xok();
			}
			None => handle,
		};
		let name = handle.describe();
		let document = match handle.read().await {
			Ok(document) => document,
			Err(err) => return self.fail(format!("{err}")).xok(),
		};
		self.pass(format!(
			"document {name}: {} group(s), {} record(s)",
			document.groups.len(),
			document.record_count()
		));
		let empty = AgeIdentityFile::default();
		let opened = match document.open(identities.unwrap_or(&empty)) {
			Ok(opened) => opened,
			Err(err) => return self.fail(format!("  {err}")).xok(),
		};
		for (group_name, group) in &document.groups {
			let members = format!("{} recipient(s)", group.recipients.len());
			if opened.drifted.iter().any(|group| group == group_name) {
				self.fail(format!(
					"  group `{group_name}`: opens, but its list changed since \
					it was sealed: run `secrets/rekey` ({members})"
				));
			} else if opened.can_open(group_name) {
				self.pass(format!("  group `{group_name}`: opens ({members})"));
			} else if opened.pending.iter().any(|group| group == group_name) {
				self.fail(format!(
					"  group `{group_name}`: lists this identity but was sealed \
					before it was added: a member runs `secrets/rekey` ({members})"
				));
			} else {
				self.note(format!(
					"  group `{group_name}`: this identity is not a member \
					({members})"
				));
			}
		}
		Ok(())
	}

	/// The report, with a failing status (a non-zero exit) on any failure.
	fn into_response(self) -> Result<Response> {
		match self.failed {
			false => Response::ok_text(self.lines),
			true => Response::from_status_body(
				StatusCode::INTERNAL_SERVER_ERROR,
				self.lines,
				MediaType::Text,
			),
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
	use beet_core::prelude::*;

	/// The ledger passes on a clean document, reports a hand-edited list as
	/// drift, and passes again after `rekey`.
	#[beet_core::test]
	async fn reports_drift_until_rekeyed() {
		let mut fixture = VerbWorld::new();
		// nothing declared, nothing written
		let response =
			fixture.call(SecretsCheck, Request::get("/")).await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.unwrap_str()
			.await
			.xpect_contains("✓ identity")
			.xpect_contains("- no `<Secrets>` is declared");

		// undeclared but written: checked all the same
		fixture.set("A", "1", default()).await;
		fixture
			.call(SecretsCheck, Request::get("/"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains(
				"✓ document `secrets` (secrets.toml): 1 group(s), 1 record(s)",
			);
		fixture
			.world
			.spawn((Secrets::default(), ChildOf(fixture.root)));
		fixture.world.flush();
		let stranger = AgeIdentity::generate();
		let mut strangers = AgeIdentityFile::default();
		strangers.push(stranger.clone());
		let mut document = fixture.document().await;
		document.groups.insert(
			"theirs".into(),
			SecretsGroup::new(vec![stranger.to_recipient()]),
		);
		document
			.set(&strangers, "theirs", "B", "2", default())
			.unwrap();
		let handle = fixture.secrets("secrets.toml");
		handle.write(&document).await.unwrap();
		let response =
			fixture.call(SecretsCheck, Request::get("/")).await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.unwrap_str()
			.await
			.xpect_contains(
				"✓ document `secrets` (secrets.toml): 2 group(s), 2 record(s)",
			)
			.xpect_contains("✓   group `default`: opens (1 recipient(s))")
			.xpect_contains(
				"-   group `theirs`: this identity is not a member",
			);

		// a hand edit of the list
		document
			.groups
			.get_mut("default")
			.unwrap()
			.recipients
			.push(AgeIdentity::generate().to_recipient());
		handle.write(&document).await.unwrap();
		let response =
			fixture.call(SecretsCheck, Request::get("/")).await.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("✗   group `default`: opens, but its list changed");
		fixture
			.call_str(SecretsRekey, Request::get("/"))
			.await
			.unwrap();
		fixture
			.call(SecretsCheck, Request::get("/"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);

		// a hand edit of the index
		let mut document = handle.read().await.unwrap();
		document
			.groups
			.get_mut("default")
			.unwrap()
			.secrets
			.get_mut("A")
			.unwrap()
			.note = Some("edited".into());
		handle.write(&document).await.unwrap();
		let response =
			fixture.call(SecretsCheck, Request::get("/")).await.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("`A`: the index entry differs");

		// a dated series beside an unwritten declared path checks its newest
		let series = fixture.secrets("exports/cold.toml");
		let mut document = SecretsDocument::default();
		document
			.set(&fixture.identities(), "default", "X", "1", default())
			.unwrap();
		series
			.dated(Timestamp::parse_date("2026-09-15").unwrap())
			.unwrap()
			.write(&document)
			.await
			.unwrap();
		let response = fixture
			.call(
				SecretsCheck,
				Request::from_cli_str(&format!(
					"--document={}",
					fixture.uri("exports/cold.toml")
				)),
			)
			.await
			.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.unwrap_str()
			.await
			// the uri's store is the file's directory, so the series is bare
			.xpect_contains(
				"a dated series, checking the newest (2026/09/15/000000Z.toml)",
			)
			.xpect_contains(
				"✓ document `2026/09/15/000000Z.toml`: 1 group(s), 1 record(s)",
			);

		// a named document that does not exist
		let response = fixture
			.call(
				SecretsCheck,
				Request::from_cli_str(&format!(
					"--document={}",
					fixture.uri("nope.toml")
				)),
			)
			.await
			.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("✗ document `nope.toml`: not written yet");
	}
}
