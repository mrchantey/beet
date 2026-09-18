//! `secrets/check`: the identity, and a file, verified.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Request params for [`SecretsCheck`], surfaced in `--help`.
#[derive(Reflect)]
struct CheckParams {
	/// An age file to open with the identity: a path or store uri.
	vault: Option<String>,
}

/// Verify the secrets setup: the identity resolves and, with `--vault`, the
/// file opens with it. One line per item with a tick or the reason, and a
/// non-zero exit on any failure.
///
/// ```sh
/// beet secrets/check
/// beet secrets/check --vault=infra/cert.pem.age
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("check"),
	ParamsPartial = ParamsPartial::new::<CheckParams>()
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
	if let Some(vault) = cx.input.parse_params::<CheckParams>()?.vault {
		report.vault(&vault, identities.as_ref()).await?;
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

	fn fail(&mut self, line: impl AsRef<str>) {
		writeln!(self.lines, "✗ {}", line.as_ref()).ok();
		self.failed = true;
	}

	/// One line for the file `selector` names: whether it resolves, exists
	/// and opens with `identities`.
	async fn vault(
		&mut self,
		selector: &str,
		identities: Option<&AgeIdentityFile>,
	) -> Result<()> {
		let vault = match VaultHandle::from_uri(selector) {
			Ok(vault) => vault,
			Err(err) => {
				return self.fail(format!("vault `{selector}`: {err}")).xok();
			}
		};
		let name = vault.describe();
		if !vault.exists().await? {
			return self
				.fail(format!(
					"vault {name}: not written yet (`secrets/encrypt` writes it)"
				))
				.xok();
		}
		let Some(identities) = identities else {
			return self
				.fail(format!("vault {name}: no identity to open it with"))
				.xok();
		};
		match vault.read(identities).await {
			Ok(plaintext) => self.pass(format!(
				"vault {name}: opens, {} bytes",
				plaintext.len()
			)),
			Err(err) => self.fail(format!("vault {name}: {err}")),
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

	/// A file the identity cannot open fails the check, and passes once it is
	/// encrypted to it.
	#[beet_core::test]
	async fn fails_on_a_locked_file_and_passes_when_opened() {
		let mut world = VerbWorld::new();
		let response =
			world.call(SecretsCheck, Request::get("/")).await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response.unwrap_str().await.xpect_contains("✓ identity");

		let stranger = AgeIdentity::generate().to_recipient();
		world
			.vault("cert.pem.age")
			.write(b"a = 1\n", &[stranger])
			.await
			.unwrap();
		let uri = world.uri("cert.pem.age");
		let request = || Request::from_cli_str(&format!("--vault={uri}"));
		let response = world.call(SecretsCheck, request()).await.unwrap();
		response
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("✓ identity")
			.xpect_contains("✗ vault `cert.pem.age`");

		world.write("cert.pem.age", "a = 1\n").await;
		world
			.call(SecretsCheck, request())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
