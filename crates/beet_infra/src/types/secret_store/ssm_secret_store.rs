//! AWS parameter store as a stack's secret store: the declaration every
//! stack carries by default, its attach, and the provider over the `aws`
//! cli.
//!
//! The cli rather than the SDK because a deploy already depends on it (the log
//! tail, the reverse-dns request, the SES probe) and because parameter store
//! is five verbs: adding an SDK client for them would be more surface than
//! the feature it buys. The provider is therefore native and `deploy`; the
//! declaration registers on every target, since a lean binary's document
//! still carries the tag.
//!
//! Nothing here logs a value. A `SecureString` that reaches a terminal is a
//! `SecureString` that reaches a scrollback buffer, a CI log and whatever
//! ingests it.

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use serde_json::Value;

/// Declares that the stack's secrets live in AWS parameter store, in the
/// stack's region: `<SsmSecrets/>` under a `<Stack>`, and the declaration a
/// stack with none is taken to carry. Absolute, whatever the launch: every
/// stack verb reaches the cloud (an apply, a provision) and addresses the
/// declaration's meaning, exactly as a sync addresses a bucket's
/// `store_uri` and never its runtime stand-in, so a deploy can never mint a
/// stage's secrets somewhere its box does not read. A stand-in for a run
/// with no account is an explicit `<DocumentSecrets path="target/.."/>`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SsmSecrets;

impl SsmSecrets {
	/// The store this declaration means for `stack`: parameter store in its
	/// region (native and `deploy`, since the provider drives the `aws`
	/// cli), an error naming the feature otherwise.
	pub fn store(stack: ResolvedStack) -> Result<SecretStore> {
		cfg_if! {
			if #[cfg(all(feature = "deploy", not(target_arch = "wasm32")))] {
				SecretStore::new(SsmSecretStore::new(stack)).xok()
			} else {
				bevybail!(
					"stack `{}--{}` keeps its secrets in parameter store and this \
					build has no provider for it (the `deploy` feature, native): \
					declare `<DocumentSecrets path=\"..\"/>` under the stack",
					stack.app_name(),
					stack.stage()
				)
			}
		}
	}
}

/// Observer: land the [`SecretStore`] an `<SsmSecrets/>` declares on its
/// entity, [`SsmSecrets::store`] for the stack it is declared under.
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
			entity.insert(SsmSecrets::store(stack)?);
			Ok(())
		});
}


/// The [`SecretStore`] over AWS parameter store, scoped to one stack in its
/// region: every secret a `SecureString` at `/app/stage/label`
/// ([`SecretRef::name`]), the note and rotation its description
/// ([`SecretRef::description`]), encrypted under the account's `aws/ssm`
/// key, which authorises account principals through its own key policy so a
/// reader needs no `kms:` grant. What `<SsmSecrets/>` (declared or implicit)
/// attaches.
///
/// A stack's secrets nest under one prefix ([`SecretRef::prefix`]), which is
/// what an instance role grants in one statement and what
/// [`list`](SecretStoreProvider::list) walks; another stack of the same
/// region is [`for_stack`](SecretStoreProvider::for_stack) away.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
#[derive(Debug, Clone)]
pub struct SsmSecretStore {
	stack: ResolvedStack,
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl SsmSecretStore {
	/// The provider id.
	pub const ID: &'static str = "ssm";

	/// The store over parameter store in `stack`'s region, scoped to it.
	pub fn new(stack: ResolvedStack) -> Self { Self { stack } }

	/// The region every request goes to.
	fn region(&self) -> &str { self.stack.region() }

	/// The directory this stack's secrets sit under.
	fn prefix(&self) -> String { SecretRef::prefix(&self.stack) }

	/// Read a parameter, decrypting a `SecureString`. `Ok(None)` when it does
	/// not exist; any other failure (no credentials, no permission) is an
	/// error rather than a silent mint of a second secret.
	async fn get_parameter(&self, name: &str) -> Result<Option<String>> {
		let output = aws_cli_ext::ssm(self.region(), [
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
		let result = aws_cli_ext::ssm(self.region(), args)
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
		let body = aws_cli_ext::ssm(self.region(), [
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
		let body = aws_cli_ext::ssm(self.region(), [
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
				self.region(),
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

	/// The label of a parameter under this stack's prefix, ie the
	/// `db-password` of `/app/stage/db-password`.
	fn label_of(&self, name: &str) -> Option<SecretRef> {
		name.strip_prefix(&format!("{}/", self.prefix()))
			.filter(|label| !label.is_empty())
			.map(SecretRef::new)
	}

	/// One listed parameter as an entry.
	fn entry(&self, item: &Value) -> Option<SecretEntry> {
		let name = item["Name"].as_str()?;
		let (note, rotation) = SecretRef::parse_description(
			item["Description"].as_str().unwrap_or_default(),
		);
		SecretEntry {
			secret: self.label_of(name)?,
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

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl SecretStoreProvider for SsmSecretStore {
	fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { Self::ID }

	fn region(&self) -> Option<SmolStr> { Some(self.stack.region().clone()) }

	fn stack(&self) -> &ResolvedStack { &self.stack }

	/// The same region serves every stack, so a rescope is one.
	fn for_stack(
		&self,
		stack: &ResolvedStack,
	) -> Result<Box<dyn SecretStoreProvider>> {
		let store: Box<dyn SecretStoreProvider> =
			Box::new(Self::new(stack.clone()));
		store.xok()
	}

	/// The parameter name, ie `/beetmash/prod/db-password`.
	fn address(&self, secret: &SecretRef) -> SmolStr {
		secret.name(&self.stack).into()
	}

	fn get(
		&self,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let this = self.clone();
		Box::pin(
			async move { this.get_parameter(&secret.name(&this.stack)).await },
		)
	}

	fn create(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.put_parameter(&secret.name(&this.stack), &value, &meta, false)
				.await
		})
	}

	fn overwrite(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			this.put_parameter(&secret.name(&this.stack), &value, &meta, true)
				.await
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
		let this = self.clone();
		Box::pin(async move {
			this.describe_parameters(&this.prefix())
				.await?
				.iter()
				.filter_map(|item| this.entry(item))
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// The descriptions from one listing and the values from one decrypting
	/// read, joined by name.
	fn read_all(&self) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.clone();
		Box::pin(async move {
			let prefix = this.prefix();
			let entries = this.describe_parameters(&prefix).await?;
			let values = this.get_parameters_by_path(&prefix).await?;
			values
				.iter()
				.filter_map(|item| {
					let mut entry = this.entry(item)?;
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
		secrets: Vec<SecretRef>,
	) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
		let this = self.clone();
		Box::pin(async move {
			let names = secrets
				.iter()
				.map(|secret| secret.name(&this.stack))
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

#[cfg(all(test, feature = "deploy", not(target_arch = "wasm32")))]
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
		let store = SsmSecretStore::new(stack());
		store
			.address(&SecretRef::new("db-password"))
			.as_str()
			.xpect_eq("/beetmash/prod/db-password");
		store
			.address(&SecretRef::new("mail-admin-password"))
			.as_str()
			.xpect_eq("/beetmash/prod/mail-admin-password");
		SecretStoreProvider::region(&store)
			.unwrap()
			.xpect_eq(stack().region().clone());
		// a rescope keeps the region and composes the other stack's names
		let drill = Stack::new("beetmash")
			.with_stage("drill")
			.resolve(&PackageConfig::default());
		store
			.for_stack(&drill)
			.unwrap()
			.address(&SecretRef::new("db-password"))
			.as_str()
			.xpect_eq("/beetmash/drill/db-password");
	}

	/// A listing reads labels back out of names under the stack's own
	/// prefix and nothing else, with the description as the note.
	#[beet_core::test]
	fn entries_read_labels_notes_and_dates() {
		let store = SsmSecretStore::new(stack());
		let entry = store
			.entry(&json!({
				"Name": "/beetmash/prod/dkim-example-com",
				"Description": "manual:a new selector :: the signing key",
				"LastModifiedDate": "2026-09-15T01:01:01.500000+00:00",
			}))
			.unwrap();
		entry.secret.label().as_str().xpect_eq("dkim-example-com");
		entry
			.address
			.as_str()
			.xpect_eq("/beetmash/prod/dkim-example-com");
		entry.note.unwrap().as_str().xpect_eq("the signing key");
		entry
			.rotation
			.xpect_eq(Some(SecretRotation::manual("a new selector")));
		entry
			.modified
			.unwrap()
			.format_iso8601()
			.xpect_eq("2026-09-15T01:01:01.500Z");
		// a neighbouring stage is never a label of this one
		store
			.entry(&json!({"Name": "/beetmash/prod-two/x", "Description": ""}))
			.xpect_none();
		SsmSecretStore::under("/beetmash/prod", vec![
			json!({"Name": "/beetmash/prod/x"}),
			json!({"Name": "/beetmash/prod-two/y"}),
		])
		.len()
		.xpect_eq(1);
	}
}
