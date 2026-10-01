//! `secrets/exec`: a child command run with a document's env vars.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsExec`], surfaced in `--help`.
#[derive(Reflect)]
struct ExecParams {
	/// Pass only these records, comma separated; absent, every `EnvVar`
	/// record the document holds. A name the document does not hold is an
	/// error, so a typo fails here rather than as the child's `AccessDenied`.
	only: Option<String>,
	/// The command and its arguments, everything after `--`.
	nested_args: Vec<String>,
}

/// Run a command with a document's `EnvVar` records in its environment,
/// the `sops exec-env` shape: for `tofu` by hand during a recovery, or any
/// tool that reads credentials from its environment. The values reach the
/// child only, never argv or a log (each is redacted from the child's
/// reported output), and the child's exit code is this command's.
///
/// ```sh
/// beet secrets/exec -- tofu plan
/// beet secrets/exec --document=mail-prod -- aws sts get-caller-identity
/// beet secrets/exec --only=AWS_ACCESS_KEY_ID,AWS_SECRET_ACCESS_KEY -- aws s3 ls
/// ```
///
/// ## The child is a foreign tool
///
/// This launch's own configuration is stripped from the child
/// (`without_launch_env`), unconditionally and with no flag to keep it: the
/// child of this verb is a foreign tool by definition, and a beet child that
/// belongs to THIS launch is `with_bootstrap`'s job, which hands over a config
/// constructed field by field. Without the strip another repo's beet binary
/// run this way inherits `BEET_WORKSPACE_ROOT` and every `BEET_*` knob, so it
/// resolves THIS repo's workspace and addresses THIS repo's stacks.
///
/// What the strip does NOT cover is a record this launch's own document
/// already put in the environment, because that is what most callers are here
/// for: `secrets/exec -- aws s3 sync` wants this repo's deployer pair. The
/// consequence is that a call naming ANOTHER document inherits this one's
/// records for every name the other does not hold, which for
/// `TF_STATE_PASSPHRASE` means a tofu verb addressing another repo's state
/// under the wrong passphrase. `--only` is the answer where that matters: it
/// hands over the records the tool needs by name, so what the other document
/// lacks is a loud absence rather than a wrong value.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("exec"),
	ParamsPartial = ParamsPartial::new::<(DocumentParams, ExecParams)>()
)]
pub async fn SecretsExec(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<ExecParams>()?;
	let Some((command, args)) = params.nested_args.split_first() else {
		bevybail!(
			"a command is required after `--`, ie `secrets/exec -- tofu plan`"
		);
	};
	let handle = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let pairs = SecretsExec::narrow(
		handle
			.read()
			.await?
			.open(&AgeIdentityFile::require()?)?
			.env_vars(),
		params.only.as_deref(),
		&handle.describe(),
	)?;
	info!(
		"running `{command}` with {} variable(s) from {}",
		pairs.len(),
		handle.describe()
	);
	let process = pairs
		.iter()
		.fold(
			ChildProcess::new(command.as_str()),
			|process, (_, value)| process.with_secret(value.clone()),
		)
		.without_launch_env()
		.with_args(args.iter().map(String::as_str))
		.with_envs(pairs);
	let status = process.spawn()?.status().await?;
	match status.success() {
		true => Response::ok().xok(),
		false => {
			bevybail!("`{command}` exited with {}", status.code().unwrap_or(-1))
		}
	}
}

impl SecretsExec {
	/// `pairs` narrowed to the `--only` list, in the list's own order; absent
	/// a list, every pair. Every name the document holds no `EnvVar` record
	/// for is reported at once, since the alternative is a child that silently
	/// runs without the credential it was asked to carry, and one typo per run
	/// is one run per typo.
	pub(crate) fn narrow(
		pairs: Vec<(SmolStr, SmolStr)>,
		only: Option<&str>,
		document: &str,
	) -> Result<Vec<(SmolStr, SmolStr)>> {
		let Some(only) = only else {
			return pairs.xok();
		};
		let names = str_ext::csv(only).collect::<Vec<_>>();
		if names.is_empty() {
			bevybail!(
				"`--only` names no record, so the child would carry none: \
				drop the flag to carry every record of {document}"
			);
		}
		let missing = names
			.iter()
			.filter(|name| !pairs.iter().any(|(key, _)| key == **name))
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			bevybail!(
				"`--only` names {}, which {document} holds no `EnvVar` record \
				for; it holds {}",
				missing
					.iter()
					.map(|name| format!("`{name}`"))
					.collect::<Vec<_>>()
					.join(", "),
				match pairs.is_empty() {
					true => "none".to_string(),
					false => pairs
						.iter()
						.map(|(key, _)| key.as_str())
						.collect::<Vec<_>>()
						.join(", "),
				}
			);
		}
		names
			.into_iter()
			.filter_map(|name| {
				pairs.iter().find(|(key, _)| key == name).cloned()
			})
			.collect::<Vec<_>>()
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::vault::test_support::VerbWorld;
	use beet_core::prelude::*;

	/// The child sees the document's env vars and nothing roleless; a
	/// failing child fails the verb.
	#[beet_core::test]
	async fn runs_the_child_with_the_env_vars() {
		let mut fixture = VerbWorld::new();
		fixture
			.set("BEET_TEST_EXEC_VAR", "seen", SecretRecord {
				role: Some(SecretRole::EnvVar),
				..default()
			})
			.await;
		fixture
			.set("BEET_TEST_EXEC_KEPT", "hidden", default())
			.await;
		fixture
			.call(
				SecretsExec,
				Request::from_cli_str(
					"-- sh -c 'test \"$BEET_TEST_EXEC_VAR\" = seen && test -z \"$BEET_TEST_EXEC_KEPT\"'",
				),
			)
			.await
			.unwrap();
		fixture
			.call(SecretsExec, Request::from_cli_str("-- sh -c 'exit 3'"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("exited with 3");
		fixture
			.call(SecretsExec, Request::get("/"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("after `--`");
	}

	/// The child is a foreign tool, so this launch's own configuration does
	/// not reach it while the document's records still do. `BEET_WORKSPACE_ROOT` is
	/// the assertion that bites: cargo's `[env]` block sets it for every
	/// process it spawns, this one included, and a beet child inheriting it
	/// resolves the wrong workspace.
	#[beet_core::test]
	async fn strips_this_launch_from_the_child() {
		// the child's empty value only means something because the parent has one
		env_ext::var(fs_ext::WORKSPACE_ROOT).unwrap();
		let mut fixture = VerbWorld::new();
		fixture
			.set("BEET_TEST_STRIP_VAR", "seen", SecretRecord {
				role: Some(SecretRole::EnvVar),
				..default()
			})
			.await;
		fixture
			.call(
				SecretsExec,
				Request::from_cli_str(
					"-- sh -c 'test -z \"$BEET_WORKSPACE_ROOT\" && \
					test \"$BEET_TEST_STRIP_VAR\" = seen'",
				),
			)
			.await
			.unwrap();
	}

	/// `--only` passes the records it names and nothing else, and a name the
	/// document does not hold fails here rather than in the child.
	#[beet_core::test]
	async fn only_narrows_the_records() {
		let mut fixture = VerbWorld::new();
		for name in ["BEET_TEST_ONLY_A", "BEET_TEST_ONLY_B"] {
			fixture
				.set(name, "seen", SecretRecord {
					role: Some(SecretRole::EnvVar),
					..default()
				})
				.await;
		}
		fixture
			.call(
				SecretsExec,
				Request::from_cli_str(
					"--only=BEET_TEST_ONLY_A -- sh -c \
					'test \"$BEET_TEST_ONLY_A\" = seen && test -z \"$BEET_TEST_ONLY_B\"'",
				),
			)
			.await
			.unwrap();
		// every bad name at once, so two typos take one run
		fixture
			.call(
				SecretsExec,
				Request::from_cli_str("--only=NOPE,ALSO_NOPE -- sh -c true"),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`NOPE`, `ALSO_NOPE`")
			.xpect_contains("BEET_TEST_ONLY_A");
		// an explicit narrowing that names nothing is a typo, not a request
		fixture
			.call(SecretsExec, Request::from_cli_str("--only= -- sh -c true"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("names no record");
	}
}
