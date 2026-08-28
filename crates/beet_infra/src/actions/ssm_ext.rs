//! Parameter store reads and writes, over the `aws` cli.
//!
//! The cli rather than the SDK because a deploy already depends on it (the log
//! tail, the reverse-dns request, the SES probe) and because parameter store is
//! four verbs: adding an SDK client for them would be more surface than the
//! feature it buys.
//!
//! Nothing here logs a value. A `SecureString` that reaches a terminal is a
//! `SecureString` that reaches a scrollback buffer, a CI log and whatever
//! ingests it.
use beet_core::prelude::*;

/// Read a parameter, decrypting a `SecureString`. `Ok(None)` when it does not
/// exist, which is the case every create-if-missing caller acts on; any other
/// failure (no credentials, no permission) is an error rather than a silent
/// mint of a second secret.
pub async fn get(region: &str, name: &str) -> Result<Option<String>> {
	let output = command(region, [
		"get-parameter",
		"--name",
		name,
		"--with-decryption",
		"--query",
		"Parameter.Value",
		"--output",
		"text",
	])
	.run_async()
	.await;
	match output {
		Ok(output) => String::from_utf8_lossy(&output.stdout)
			.trim_end_matches('\n')
			.to_string()
			.xmap(Some)
			.xok(),
		Err(err) => match err.to_string().contains("ParameterNotFound") {
			true => Ok(None),
			false => Err(err),
		},
	}
}

/// Create a `SecureString` parameter, failing if one already exists.
///
/// Deliberately not `--overwrite`: two deploys racing to mint the same secret
/// must not each believe theirs is the one in use, and the loser re-reads the
/// winner's value instead. Encrypted under the account's `aws/ssm` key, which
/// authorises account principals through its own key policy, so the reader
/// needs no `kms:` grant.
pub async fn create(region: &str, name: &str, value: &str) -> Result {
	command(region, [
		"put-parameter",
		"--name",
		name,
		"--type",
		"SecureString",
		"--value",
		value,
	])
	// a failed command reports its own argv, so without this the one write
	// that carries a secret is also the one most likely to print it
	.with_secret(value)
	.run_async()
	.await?;
	Ok(())
}

/// Whether `err` is the put-parameter conflict, ie somebody else won the race.
pub fn is_already_exists(err: &BevyError) -> bool {
	err.to_string().contains("ParameterAlreadyExists")
}

/// Create or replace a `SecureString` parameter.
///
/// The opposite posture to [`create`], for the opposite ownership: use this
/// only for a value some OTHER system mints and this parameter mirrors (the
/// bootstrap admin credential a fresh Stalwart returns exactly once), where an
/// existing value is by definition stale. A secret this deploy generates goes
/// through [`create`], whose refusal to overwrite is what makes racing deploys
/// safe.
pub async fn overwrite(region: &str, name: &str, value: &str) -> Result {
	command(region, [
		"put-parameter",
		"--name",
		name,
		"--type",
		"SecureString",
		"--value",
		value,
		"--overwrite",
	])
	// a failed command reports its own argv, so without this the one write
	// that carries a secret is also the one most likely to print it
	.with_secret(value)
	.run_async()
	.await?;
	Ok(())
}

/// An `aws ssm` invocation in `region`. Drops a possibly-empty inherited
/// `AWS_PROFILE`, which the cli reads as a profile literally named `""`.
fn command<'a>(
	region: &str,
	args: impl IntoIterator<Item = &'a str>,
) -> ChildProcess {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args(
			["ssm"]
				.into_iter()
				.chain(args)
				.map(SmolStr::from)
				.chain([SmolStr::from("--region"), SmolStr::from(region)]),
		)
}
