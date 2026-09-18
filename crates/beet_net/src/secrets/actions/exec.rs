//! `secrets/exec`: a child command run with a document's env vars.

use super::DocumentParams;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsExec`], surfaced in `--help`.
#[derive(Reflect)]
struct ExecParams {
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
/// beet secrets/exec --vault=mail-prod -- aws sts get-caller-identity
/// ```
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
	let vault = DocumentParams::resolve(&cx.input, &cx.caller).await?;
	let pairs = vault
		.read_document()
		.await?
		.open(&AgeIdentityFile::require()?)?
		.env_vars();
	info!(
		"running `{command}` with {} variable(s) from {}",
		pairs.len(),
		vault.describe()
	);
	let process = pairs
		.iter()
		.fold(
			ChildProcess::new(command.as_str()),
			|process, (_, value)| process.with_secret(value.clone()),
		)
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

#[cfg(test)]
mod test {
	use super::super::test_support::VerbWorld;
	use crate::prelude::*;
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
}
