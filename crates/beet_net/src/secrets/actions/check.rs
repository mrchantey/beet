//! `secrets/check`: the identity and every vault, verified.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Verify the secrets setup: the identity resolves, every declared vault
/// (and the entry's `.env.age` when present) opens with it, and each list
/// of recipients includes this identity's own. One line per item with a
/// tick or the reason, and a non-zero exit on any failure; a list missing
/// your own recipient is a warning, since the next write would lock you
/// out.
///
/// ```sh
/// beet secrets/check
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(PathPartial = PathPartial::new("check"))]
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
	let own = identities
		.as_ref()
		.map(AgeIdentityFile::recipients)
		.unwrap_or_default();

	// every declared vault, plus the undeclared `.env.age` beside the entry
	let mut vaults = VaultHandle::declared(&cx.caller).await?;
	if !vaults.iter().any(|(label, _)| label == Vault::ENV_LABEL)
		&& let Ok(env) = VaultHandle::resolve(&cx.caller, None).await
		&& env.exists().await?
	{
		vaults.push((SmolStr::new(Vault::ENV_LABEL), Ok(env)));
	}
	if vaults.is_empty() {
		report.pass("vaults: none declared and no `.env.age` beside the entry");
	}
	for (label, vault) in vaults {
		let vault = match vault {
			Ok(vault) => vault,
			Err(err) => {
				report.fail(format!("vault `{label}`: {err}"));
				continue;
			}
		};
		let name = vault.describe();
		if !vault.exists().await? {
			report.fail(format!(
				"vault {name}: not written yet (`secrets/set` or \
				`secrets/import` writes it)"
			));
			continue;
		}
		let Some(identities) = &identities else {
			report.fail(format!("vault {name}: no identity to open it with"));
			continue;
		};
		match vault.read(identities).await {
			Ok(doc) => report
				.pass(format!("vault {name}: {} key(s)", doc.keys().len())),
			Err(err) => {
				report.fail(format!("vault {name}: {err}"));
				continue;
			}
		}
		match &vault.recipients {
			Some(recipients) if recipients.is_empty() => report.fail(format!(
				"vault {name}: declares no recipients, so nothing can write it: \
				name them on the `<Vault>` or an ancestor `{{AgeRecipients}}`"
			)),
			Some(recipients)
				if !own.iter().any(|mine| recipients.contains(mine)) =>
			{
				report.warn(format!(
					"vault {name}: none of your recipients is in its list, so \
					the next write locks you out"
				))
			}
			Some(_) => {}
			None => report.warn(format!(
				"vault {name}: undeclared, so a write encrypts it to this \
				machine's identity alone"
			)),
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

	fn warn(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "! {}", line.as_ref()).ok();
	}

	fn fail(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "✗ {}", line.as_ref()).ok();
		self.failed = true;
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

	/// A vault the identity cannot open fails the check, and passes once
	/// rekeyed to it.
	#[beet_core::test]
	async fn fails_on_a_locked_vault_and_passes_after_rekey() {
		let mut world = VerbWorld::new();
		world.set("mail", "a", "1").await;
		world.set("notes", "b", "2").await;
		let response =
			world.call(SecretsCheck, Request::get("/")).await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.unwrap_str()
			.await
			.xpect_contains("✓ identity")
			.xpect_contains("✓ vault `mail`")
			.xpect_contains("✓ vault `notes`");

		// `mail` re-encrypted to a stranger
		let stranger = AgeIdentity::generate().to_recipient();
		world
			.call_str(
				SecretsRekey,
				Request::from_cli_str(&format!(
					"--vault=mail --recipients={stranger}"
				)),
			)
			.await
			.unwrap();
		let response =
			world.call(SecretsCheck, Request::get("/")).await.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("✗ vault `mail`")
			.xpect_contains("✓ vault `notes`");

		// the other human rekeys it back to the declared list
		let vault =
			VaultHandle::new(world.store.clone(), "secrets/mail.toml.age")
				.unwrap();
		let mut doc = VaultDocument::new(VaultFormat::Toml);
		doc.set("a", "1").unwrap();
		vault
			.write(&doc, &[world.identity.to_recipient()])
			.await
			.unwrap();
		world
			.call(SecretsCheck, Request::get("/"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
