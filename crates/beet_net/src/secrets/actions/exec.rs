//! `secrets/exec`: a child command run with an env vault in its environment.

use super::VaultParams;
use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`SecretsExec`], surfaced in `--help`.
#[derive(Reflect)]
struct ExecParams {
	/// The command and its arguments, everything after `--`.
	nested_args: Vec<String>,
}

/// Run a command with an env vault loaded into its environment, the
/// `sops exec-env` shape: for `tofu` by hand during a recovery, or any tool
/// that reads credentials from its environment. The values reach the child
/// only, never argv or a log, and the child's exit code is this command's.
///
/// ```sh
/// beet secrets/exec -- tofu plan
/// beet secrets/exec --vault=mail-prod.env.age -- aws sts get-caller-identity
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("exec"),
	ParamsPartial = ParamsPartial::new::<(VaultParams, ExecParams)>()
)]
pub async fn SecretsExec(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<ExecParams>()?;
	let Some((command, args)) = params.nested_args.split_first() else {
		bevybail!(
			"a command is required after `--`, ie `secrets/exec -- tofu plan`"
		);
	};
	let vault = VaultParams::resolve(&cx.input, &cx.caller).await?;
	let doc = vault.read(&AgeIdentityFile::require()?).await?;
	let pairs = doc.as_env()?.pairs();
	info!(
		"running `{command}` with {} variable(s) from vault {}",
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

	/// The child sees the vault's pairs; a failing child fails the verb.
	#[beet_core::test]
	async fn runs_the_child_with_the_vault() {
		let mut world = VerbWorld::new();
		world.set(".env", "BEET_TEST_EXEC_VAR", "seen").await;
		world
			.call(
				SecretsExec,
				Request::from_cli_str(
					"-- sh -c 'test \"$BEET_TEST_EXEC_VAR\" = seen'",
				),
			)
			.await
			.unwrap();
		world
			.call(SecretsExec, Request::from_cli_str("-- sh -c 'exit 3'"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("exited with 3");
		world
			.call(SecretsExec, Request::get("/"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("after `--`");
	}
}
