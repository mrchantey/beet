//! AWS parameter store as a stack's secret store, over the `aws` cli.
//!
//! The cli rather than the SDK because a deploy already depends on it (the log
//! tail, the reverse-dns request, the SES probe) and because parameter store
//! is five verbs: adding an SDK client for them would be more surface than
//! the feature it buys.
//!
//! Nothing here logs a value. A `SecureString` that reaches a terminal is a
//! `SecureString` that reaches a scrollback buffer, a CI log and whatever
//! ingests it.

use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;

/// The [`SecretStore`] over AWS parameter store in one region: every secret
/// a `SecureString` at `/app/stage/label` ([`SecretRef::name`]), the note
/// and rotation its description ([`SecretRef::description`]), encrypted
/// under the account's `aws/ssm` key, which authorises account principals
/// through its own key policy so a reader needs no `kms:` grant. The default
/// store of a [`Remote`](ServiceAccess::Remote) launch, declared explicitly
/// as `<SsmSecrets/>`.
///
/// A stack's secrets nest under one prefix ([`SecretRef::prefix`]), which is
/// what an instance role grants in one statement and what
/// [`list`](SecretStoreProvider::list) walks.
#[derive(Debug, Clone)]
pub struct SsmSecretStore {
	region: SmolStr,
}

impl SsmSecretStore {
	/// The provider id.
	pub const ID: &'static str = "ssm";

	/// The store over parameter store in `region`.
	pub fn new(region: impl Into<SmolStr>) -> Self {
		Self {
			region: region.into(),
		}
	}

	/// The store in `stack`'s region.
	pub fn for_stack(stack: &ResolvedStack) -> Self {
		Self::new(stack.region().clone())
	}

	/// Read a parameter, decrypting a `SecureString`. `Ok(None)` when it does
	/// not exist; any other failure (no credentials, no permission) is an
	/// error rather than a silent mint of a second secret.
	async fn get_parameter(&self, name: &str) -> Result<Option<String>> {
		let output = aws_cli_ext::ssm(&self.region, [
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

	/// Write a `SecureString`, with or without `--overwrite`; the create
	/// conflict is answered as [`SecretStoreError::AlreadyExists`].
	async fn put_parameter(
		&self,
		name: &str,
		value: &str,
		meta: &SecretMeta,
		overwrite: bool,
	) -> Result {
		let mut args = vec![
			"put-parameter",
			"--name",
			name,
			"--type",
			"SecureString",
			"--value",
			value,
		];
		let description = SecretRef::description(
			meta.note.as_deref(),
			meta.rotation.as_ref(),
		);
		if !description.is_empty() {
			args.extend(["--description", &description]);
		}
		if overwrite {
			args.push("--overwrite");
		}
		let result = aws_cli_ext::ssm(&self.region, args)
			// a failed command reports its own argv, so without this the one
			// write that carries a secret is also the one most likely to print it
			.with_secret(value)
			.run_async()
			.await;
		match result {
			Ok(_) => Ok(()),
			Err(err) if err.to_string().contains("ParameterAlreadyExists") => {
				Err(SecretStoreError::AlreadyExists {
					address: name.into(),
				}
				.into())
			}
			Err(err) => Err(err),
		}
	}

	/// Every parameter's metadata under `prefix`, recursively, without a
	/// value: `describe-parameters` is the one call that reports the
	/// description. Names are filtered against `prefix/` as well as the
	/// api's own path filter, so a stage whose name is a prefix of another's
	/// (`drill`, `drill-two`) can never list its neighbour on an api subtlety.
	async fn describe_parameters(&self, prefix: &str) -> Result<Vec<Value>> {
		let body = aws_cli_ext::ssm(&self.region, [
			"describe-parameters",
			"--parameter-filters",
			&format!("Key=Path,Option=Recursive,Values={prefix}"),
			"--query",
			"Parameters[].{Name:Name,Description:Description,LastModifiedDate:LastModifiedDate}",
			"--output",
			"json",
		])
		.run_async_stdout()
		.await?;
		Self::under(prefix, Self::parse_list(&body)?).xok()
	}

	/// Every parameter under `prefix` with its value, decrypted, in one call.
	async fn get_parameters_by_path(&self, prefix: &str) -> Result<Vec<Value>> {
		let body = aws_cli_ext::ssm(&self.region, [
			"get-parameters-by-path",
			"--path",
			prefix,
			"--recursive",
			"--with-decryption",
			"--query",
			"Parameters[].{Name:Name,Value:Value,LastModifiedDate:LastModifiedDate}",
			"--output",
			"json",
		])
		.run_async_stdout()
		.await?;
		Self::under(prefix, Self::parse_list(&body)?).xok()
	}

	/// Delete parameters by name, in the api's batches of ten, answering the
	/// names actually deleted: one already gone is reported under
	/// `InvalidParameters` and simply absent here.
	async fn delete_parameters(&self, names: &[String]) -> Result<Vec<String>> {
		let mut deleted = Vec::new();
		for batch in names.chunks(10) {
			let output = aws_cli_ext::ssm(
				&self.region,
				["delete-parameters", "--names"]
					.into_iter()
					.chain(batch.iter().map(String::as_str))
					.chain([
						"--query",
						"DeletedParameters",
						"--output",
						"json",
					]),
			)
			.run_async_stdout()
			.await?;
			deleted.extend(serde_json::from_str::<Vec<String>>(&output)?);
		}
		Ok(deleted)
	}

	/// An empty result prints nothing at all rather than `[]`.
	fn parse_list(body: &str) -> Result<Vec<Value>> {
		match body.trim().is_empty() {
			true => Vec::new().xok(),
			false => serde_json::from_str::<Vec<Value>>(body)?.xok(),
		}
	}

	/// The items whose `Name` sits under `prefix/`.
	fn under(prefix: &str, items: Vec<Value>) -> Vec<Value> {
		let under = format!("{}/", prefix.trim_end_matches('/'));
		items
			.into_iter()
			.filter(|item| {
				item["Name"]
					.as_str()
					.is_some_and(|name| name.starts_with(&under))
			})
			.collect()
	}

	/// The label of a parameter under `stack`'s prefix, ie the
	/// `db-password` of `/app/stage/db-password`.
	fn label_of(stack: &ResolvedStack, name: &str) -> Option<SecretRef> {
		name.strip_prefix(&format!("{}/", SecretRef::prefix(stack)))
			.filter(|label| !label.is_empty())
			.map(SecretRef::new)
	}

	/// One listed parameter as an entry.
	fn entry(stack: &ResolvedStack, item: &Value) -> Option<SecretEntry> {
		let name = item["Name"].as_str()?;
		let (note, rotation) = SecretRef::parse_description(
			item["Description"].as_str().unwrap_or_default(),
		);
		SecretEntry {
			secret: Self::label_of(stack, name)?,
			address: name.into(),
			note,
			rotation,
			// the cli prints an offset form, `2026-09-15T00:00:00.123000+00:00`
			modified: item["LastModifiedDate"]
				.as_str()
				.and_then(Timestamp::parse_rfc3339),
		}
		.xmap(Some)
	}
}

impl SecretStoreProvider for SsmSecretStore {
	fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { Self::ID }

	fn region(&self) -> Option<SmolStr> { Some(self.region.clone()) }

	/// The parameter name, ie `/beetmash/prod/db-password`.
	fn address(&self, stack: &ResolvedStack, secret: &SecretRef) -> SmolStr {
		secret.name(stack).into()
	}

	fn get(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let this = self.clone();
		Box::pin(async move { this.get_parameter(&secret.name(&stack)).await })
	}

	fn create(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.put_parameter(&secret.name(&stack), &value, &meta, false)
				.await
		})
	}

	fn overwrite(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.put_parameter(&secret.name(&stack), &value, &meta, true)
				.await
		})
	}

	fn list(
		&self,
		stack: ResolvedStack,
	) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
		let this = self.clone();
		Box::pin(async move {
			this.describe_parameters(&SecretRef::prefix(&stack))
				.await?
				.iter()
				.filter_map(|item| Self::entry(&stack, item))
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// The descriptions from one listing and the values from one decrypting
	/// read, joined by name.
	fn read_all(
		&self,
		stack: ResolvedStack,
	) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.clone();
		Box::pin(async move {
			let prefix = SecretRef::prefix(&stack);
			let entries = this.describe_parameters(&prefix).await?;
			let values = this.get_parameters_by_path(&prefix).await?;
			values
				.iter()
				.filter_map(|item| {
					let mut entry = Self::entry(&stack, item)?;
					(entry.note, entry.rotation) = SecretRef::parse_description(
						entries
							.iter()
							.find(|described| described["Name"] == item["Name"])
							.and_then(|described| {
								described["Description"].as_str()
							})
							.unwrap_or_default(),
					);
					Some((entry, item["Value"].as_str()?.to_string()))
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}

	fn delete(
		&self,
		stack: ResolvedStack,
		secrets: Vec<SecretRef>,
	) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
		let this = self.clone();
		Box::pin(async move {
			let names = secrets
				.iter()
				.map(|secret| secret.name(&stack))
				.collect::<Vec<_>>();
			let deleted = this.delete_parameters(&names).await?;
			secrets
				.into_iter()
				.zip(names)
				.filter(|(_, name)| deleted.contains(name))
				.map(|(secret, _)| secret)
				.collect::<Vec<_>>()
				.xok()
		})
	}
}

/// Observer: land the [`SecretStore`] an `<SsmSecrets/>` declares on its
/// entity, parameter store in the region of the stack it is declared under.
/// Deferred through the command queue because the stack is an ancestor,
/// which lands after insertion.
pub(crate) fn attach_ssm_secrets(
	ev: On<Insert, SsmSecrets>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			if !entity.world().contains_resource::<PackageConfig>() {
				bevybail!(
					"resolving `<SsmSecrets/>` needs the `PackageConfig` \
					resource, which `BootstrapPlugin` inserts"
				);
			}
			let stack = entity.with_state::<StackQuery, _>(|entity, stacks| {
				stacks.resolve(entity)
			});
			entity.insert(SecretStore::new(SsmSecretStore::for_stack(&stack)));
			Ok(())
		});
}

#[cfg(test)]
mod test {
	use super::*;
	use serde_json::json;

	fn stack() -> ResolvedStack {
		Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default())
	}

	/// The composition the live boot scripts and IAM policies already carry,
	/// so the strings are pinned: a renamed parameter is a box that boots
	/// without its database password.
	#[beet_core::test]
	fn addresses_are_the_parameter_names() {
		let store = SsmSecretStore::for_stack(&stack());
		store
			.address(&stack(), &SecretRef::new("db-password"))
			.as_str()
			.xpect_eq("/beetmash/prod/db-password");
		store
			.address(&stack(), &SecretRef::new("mail-admin-password"))
			.as_str()
			.xpect_eq("/beetmash/prod/mail-admin-password");
		store.region().unwrap().xpect_eq(stack().region().clone());
	}

	/// A listing reads labels back out of names under the stack's own
	/// prefix and nothing else, with the description as the note.
	#[beet_core::test]
	fn entries_read_labels_notes_and_dates() {
		let stack = stack();
		let entry = SsmSecretStore::entry(
			&stack,
			&json!({
				"Name": "/beetmash/prod/dkim-example-com",
				"Description": "manual:a new selector :: the signing key",
				"LastModifiedDate": "2026-09-15T01:01:01.500000+00:00",
			}),
		)
		.unwrap();
		entry.secret.label().as_str().xpect_eq("dkim-example-com");
		entry
			.address
			.as_str()
			.xpect_eq("/beetmash/prod/dkim-example-com");
		entry.note.unwrap().as_str().xpect_eq("the signing key");
		entry
			.rotation
			.xpect_eq(Some(Rotation::manual("a new selector")));
		entry
			.modified
			.unwrap()
			.format_iso8601()
			.xpect_eq("2026-09-15T01:01:01.500Z");
		// a neighbouring stage is never a label of this one
		SsmSecretStore::entry(
			&stack,
			&json!({"Name": "/beetmash/prod-two/x", "Description": ""}),
		)
		.xpect_none();
		SsmSecretStore::under("/beetmash/prod", vec![
			json!({"Name": "/beetmash/prod/x"}),
			json!({"Name": "/beetmash/prod-two/y"}),
		])
		.len()
		.xpect_eq(1);
	}
}
