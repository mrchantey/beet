//! `secrets/check`: the identity and every document, verified.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Verify the secrets setup: the identity resolves; every declared document
/// (or the one `--vault` names) reads and every group this identity opens
/// verifies against its index; every group's sealed recipient list matches
/// its list (else "run `secrets/rekey`"); and which groups this identity is
/// not in. Given an age file's path instead, whether it opens. One line per
/// item with a tick or the reason; the whole ledger prints, then a non-zero
/// exit on any failure.
///
/// ```sh
/// beet secrets/check
/// beet secrets/check --vault=mail-prod
/// beet secrets/check --vault=infra/cert.pem.age
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
	let selector = cx.input.parse_params::<DocumentParams>()?.vault;
	let mut handles = match &selector {
		Some(_) => Vec::new(),
		None => VaultHandle::declared(&cx.caller).await?,
	};
	// named, or nothing declared: the one document the selector resolves,
	// which undeclared and unwritten is nothing to check
	if handles.is_empty() {
		let handle =
			VaultHandle::resolve_document(&cx.caller, selector.as_deref())
				.await;
		match (&selector, &handle) {
			(None, Ok(handle)) if !handle.exists().await? => report.note(
				"no `<Secrets>` is declared in this entry and no `secrets.toml.age` \
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
			Ok(vault) => report.vault(&vault, identities.as_ref()).await?,
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

	/// The lines for the file `vault` names: whether it exists, and as a
	/// document its ledger, as an age file whether it opens.
	async fn vault(
		&mut self,
		vault: &VaultHandle,
		identities: Option<&AgeIdentityFile>,
	) -> Result<()> {
		let name = vault.describe();
		if !vault.exists().await? {
			return self
				.fail(format!(
					"{name}: not written yet (`secrets/set` writes a document, \
					`secrets/encrypt` a file)"
				))
				.xok();
		}
		let bytes = vault.read_bytes().await?;
		if VaultHandle::is_age_file(&bytes) {
			match identities.map(|identities| identities.decrypt(&bytes)) {
				Some(Ok(plaintext)) => self.pass(format!(
					"file {name}: opens, {} bytes",
					plaintext.len()
				)),
				Some(Err(err)) => self.fail(format!("file {name}: {err}")),
				None => self
					.fail(format!("file {name}: no identity to open it with")),
			}
			return Ok(());
		}
		let document = match vault.read_document().await {
			Ok(document) => document,
			Err(err) => return self.fail(format!("{err}")).xok(),
		};
		self.pass(format!(
			"document {name}: {} group(s), {} record(s)",
			document.groups.len(),
			document.secrets.len()
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
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
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
				"✓ document `secrets` (secrets.toml.age): 1 group(s), 1 record(s)",
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
			.set(
				&strangers,
				"B",
				"2",
				SecretRecord::default().with_group("theirs"),
			)
			.unwrap();
		let vault = fixture.vault("secrets.toml.age");
		vault.write_document(&document).await.unwrap();
		let response =
			fixture.call(SecretsCheck, Request::get("/")).await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.unwrap_str()
			.await
			.xpect_contains(
				"✓ document `secrets` (secrets.toml.age): 2 group(s), 2 record(s)",
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
		vault.write_document(&document).await.unwrap();
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
		let mut document = vault.read_document().await.unwrap();
		document.secrets.get_mut("A").unwrap().note = Some("edited".into());
		vault.write_document(&document).await.unwrap();
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
	}

	/// An age file named by path fails the check when the identity cannot
	/// open it, and passes once it is encrypted to it.
	#[beet_core::test]
	async fn checks_an_age_file() {
		let mut fixture = VerbWorld::new();
		let stranger = AgeIdentity::generate().to_recipient();
		fixture
			.vault("cert.pem.age")
			.write(b"a = 1\n", &[stranger])
			.await
			.unwrap();
		let uri = fixture.uri("cert.pem.age");
		let request = || Request::from_cli_str(&format!("--vault={uri}"));
		let response = fixture.call(SecretsCheck, request()).await.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("✓ identity")
			.xpect_contains("✗ file `cert.pem.age`");
		fixture.write("cert.pem.age", "a = 1\n").await;
		fixture
			.call(SecretsCheck, request())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
