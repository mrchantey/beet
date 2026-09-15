//! The encrypted export of everything parameter store holds for a mail stack,
//! into the cold store beside the snapshots and the blobs.
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

impl MailSecretsExport {
	/// The prefix the exports are written under in the cold bucket, beside
	/// `sqlite/` and `blobs/`. Never expired: a few kilobytes per deploy, and
	/// the newest is the one a restore into a fresh account reads first.
	pub const PREFIX: &'static str = "secrets";

	/// The file suffix every export carries: a JSON document, age-encrypted.
	pub const SUFFIX: &'static str = ".json.age";

	/// The key an export taken at `now` lands under, in the snapshots' own
	/// `YYYY/MM/DD/HHMMSSZ` shape so one listing reads either.
	pub fn key_at(now: Timestamp) -> String {
		let (year, month, day) = now.civil_date();
		let secs = now.secs().rem_euclid(86_400);
		format!(
			"{}/{year:04}/{month:02}/{day:02}/{:02}{:02}{:02}Z{}",
			Self::PREFIX,
			secs / 3600,
			secs % 3600 / 60,
			secs % 60,
			Self::SUFFIX,
		)
	}

	/// The document an export holds: every parameter under the stack's own
	/// prefix, values included, with enough about the stack to put them back
	/// under the right names.
	pub fn document(
		stack: &ResolvedStack,
		now: Timestamp,
		parameters: &Value,
	) -> Value {
		json!({
			"exported_at": now.format_iso8601(),
			"stack": {
				"app_name": stack.app_name(),
				"stage": stack.stage(),
				"region": stack.region(),
			},
			"prefix": SecretRef::prefix(stack),
			"restore": "for each parameter: aws ssm put-parameter --type <type> --name <name> --value <value>",
			"parameters": parameters["Parameters"]
				.as_array()
				.into_iter()
				.flatten()
				.map(|parameter| json!({
					"name": parameter["Name"],
					"type": parameter["Type"],
					"version": parameter["Version"],
					"last_modified": parameter["LastModifiedDate"],
					"value": parameter["Value"],
				}))
				.collect::<Vec<_>>(),
		})
	}

	/// The `age` invocation: one `--recipient` per human, ciphertext to
	/// `output`, plaintext on stdin.
	pub fn age_args(recipients: &[SmolStr], output: &str) -> Vec<String> {
		["--encrypt".to_string()]
			.into_iter()
			.chain(recipients.iter().flat_map(|recipient| {
				["--recipient".to_string(), recipient.to_string()]
			}))
			.chain(["--output".to_string(), output.to_string()])
			.collect()
	}

	const AGE_NOT_FOUND: &'static str = "age is not installed, and the secrets export is encrypted with it (pacman -S age, apt install age, brew install age)";
}

/// Reads every parameter under the stack's prefix, encrypts the lot to the
/// declared recipient and uploads the result to the cold store.
/// `<MailSecretsExport recipient="age1.."/>` — export every secret the stack
/// holds, encrypted, into the cold store.
///
/// The snapshot and the blobs bring back the MAIL; this brings back the
/// ability to be the server that held it. Most of what is under the prefix is
/// re-mintable (a mailbox password is a new password, a relay user is a new
/// apply), but the sovereign DKIM private key is not: its public half is
/// published under a selector every receiver has cached, so a re-minted key is
/// a fortnight of unverifiable mail. And a relay credential a human minted by
/// hand is re-mintable only by that human. The export takes the whole prefix
/// rather than a list, so a secret added later is exported without anyone
/// remembering to add it here.
///
/// It runs on the DEPLOY machine at the end of every deploy and provision,
/// which is exactly when a secret changes: provision mints the mailbox
/// credentials, the apply mints the relay's, and nothing on the box ever
/// does. So the export is complete at the moment it matters and the box's
/// own permissions never widen to every secret in the prefix, which a nightly
/// export from the box would need.
///
/// The document never touches the disk in plaintext: it is piped into `age`
/// and only the ciphertext is written and uploaded. The recipients are public
/// keys, safe in the entry, one per human who may restore; each identity
/// lives with its human, off both clouds, which is the last rung of the
/// ladder this phase does not automate. No shared secret exists anywhere.
/// Decrypt with `age -d -i <identity> <file>`.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailSecretsExport(
	/// The age recipients (`age1..`) the export is encrypted to, any one of
	/// which decrypts it. An identity must never be parked in either cloud:
	/// an export the cloud can decrypt is a copy of the secrets, not an
	/// encrypted one.
	#[field]
	recipients: Vec<SmolStr>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	if recipients.is_empty() {
		bevybail!(
			"no recipients: `age-keygen` prints one starting `age1` per human \
			who may restore, and the identity it prints beside it stays with them"
		);
	}
	for recipient in &recipients {
		if !recipient.starts_with("age1") {
			bevybail!(
				"'{recipient}' is not an age recipient: `age-keygen` prints one \
				starting `age1`, and the identity it prints beside it stays with you"
			);
		}
	}
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let cold = ColdStore::resolve(mail.cold_store()?, &mail.stack).await?;
	let region = mail.stack.region();
	let prefix = SecretRef::prefix(&mail.stack);

	// every parameter under the prefix, decrypted. The cli paginates for us.
	let parameters: Value = aws_cli_ext::ssm(region, [
		"get-parameters-by-path",
		"--path",
		&prefix,
		"--recursive",
		"--with-decryption",
		"--output",
		"json",
	])
	.run_async_stdout()
	.await?
	.xmap(|body| serde_json::from_str(&body))?;
	let now = Timestamp::now();
	let document = MailSecretsExport::document(&mail.stack, now, &parameters);
	let count = document["parameters"].as_array().map(Vec::len).unwrap_or(0);
	if count == 0 {
		bevybail!(
			"nothing under {prefix} to export: the stack has not been deployed, \
			or the prefix is not where its secrets live"
		);
	}

	// plaintext in, ciphertext out, nothing in between on disk
	let key = MailSecretsExport::key_at(now);
	let local = mail.project.work_dir().join("secrets-export.json.age");
	ChildProcess::new("age")
		.with_not_found(MailSecretsExport::AGE_NOT_FOUND)
		.with_args(MailSecretsExport::age_args(&recipients, local.as_str()))
		.run_async_stdin(document.to_string())
		.await?;
	let upload = cold.upload(&local, &key).await;
	fs_ext::remove(&local).ok();
	upload?;
	info!(
		"{count} parameters under {prefix} exported to s3://{}/{key}, \
		encrypted to {} recipient(s)",
		cold.bucket,
		recipients.len()
	);
	Pass(cx.input).xok()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The key is date-shaped like a snapshot's, so a listing of the cold
	/// bucket reads the same either side of the prefix.
	#[beet_core::test]
	fn keys_are_dated_like_snapshots() {
		MailSecretsExport::key_at(Timestamp::parse_date("2026-09-15").unwrap())
			.xpect_eq("secrets/2026/09/15/000000Z.json.age");
		MailSecretsExport::key_at(Timestamp::from_secs(
			Timestamp::parse_date("2026-09-15").unwrap().secs() + 3661,
		))
		.xpect_eq("secrets/2026/09/15/010101Z.json.age");
	}

	/// Every recipient is named, so any one human's identity decrypts the
	/// export and no identity is ever shared.
	#[beet_core::test]
	fn every_recipient_is_named() {
		MailSecretsExport::age_args(
			&["age1aaa".into(), "age1bbb".into()],
			"/tmp/x.age",
		)
		.join(" ")
		.xpect_eq(
			"--encrypt --recipient age1aaa --recipient age1bbb --output /tmp/x.age",
		);
	}

	/// The document carries what a restore into a fresh account needs: the
	/// names, the types and the values, and the stack they belonged to.
	#[beet_core::test]
	fn the_document_carries_names_types_and_values() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let parameters = json!({"Parameters": [
			{"Name": "/beet-infra/dev/dkim-example-com", "Type": "SecureString",
			 "Value": "-----BEGIN PRIVATE KEY-----", "Version": 1,
			 "LastModifiedDate": "2026-09-15T00:00:00+00:00"},
		]});
		let document = MailSecretsExport::document(
			&stack,
			Timestamp::UNIX_EPOCH,
			&parameters,
		);
		document["prefix"]
			.as_str()
			.unwrap()
			.xpect_eq("/beet-infra/dev");
		document["stack"]["stage"].as_str().unwrap().xpect_eq("dev");
		document["parameters"][0]["name"]
			.as_str()
			.unwrap()
			.xpect_eq("/beet-infra/dev/dkim-example-com");
		document["parameters"][0]["type"]
			.as_str()
			.unwrap()
			.xpect_eq("SecureString");
		document["parameters"][0]["value"]
			.as_str()
			.unwrap()
			.xpect_contains("PRIVATE KEY");
	}
}
