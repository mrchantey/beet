//! AWS parameter store as a stack's secret store: the declaration every
//! stack carries by default, its attach, and the provider over the SSM json
//! api.
//!
//! Every call is a SigV4-signed request through beet's own http client
//! ([`AwsJsonClient`]) as the launch's credentials: a deployed box's pair,
//! or the deployer's on a deploy. No SDK client and no `aws` cli, so the
//! provider builds on every target and a box needs nothing installed to
//! read its secrets.
//!
//! Nothing here logs a value. A `SecureString` that reaches a terminal is a
//! `SecureString` that reaches a scrollback buffer, a CI log and whatever
//! ingests it.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// Declares that the stack's secrets live in AWS parameter store, in the
/// stack's region: `<Stack {SsmSecrets}>`, and the declaration a stack with
/// none is taken to carry. Absolute, whatever the launch: every stack verb
/// reaches the cloud (an apply, a provision) and addresses the
/// declaration's meaning, exactly as a sync addresses a bucket's
/// `store_uri` and never its runtime stand-in, so a deploy can never mint a
/// stage's secrets somewhere its box does not read. A stand-in for a run
/// with no account is an explicit `{DocumentSecrets{path:"target/.."}}`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SsmSecrets;

impl SsmSecrets {
	/// The store this declaration means for `stack`: parameter store in its
	/// region, an error naming the stack when it declares none.
	pub fn store(stack: ResolvedStack) -> Result<SecretStore> {
		SecretStore::new(SsmSecretStore::new(
			stack.secret_scope(),
			stack.aws_region()?.clone(),
		))
		.xok()
	}
}

/// Observer: land the [`SecretStore`] an `<SsmSecrets/>` declares on its
/// entity, [`SsmSecrets::store`] for the stack it is declared on or under.
/// Deferred through the command queue because the stack and its region are
/// ancestors, which land after insertion.
pub(crate) fn attach_ssm_secrets(
	ev: On<Insert<SsmSecrets>>,
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
			let stack =
				entity.with_state::<StackQuery, _>(|entity, stacks| {
					stacks
						.refuse_nested_store(entity, "SsmSecrets")
						.map(|_| stacks.resolve(entity))
				})?;
			entity.insert(SsmSecrets::store(stack)?);
			Ok(())
		});
}

/// The [`SecretStore`] over AWS parameter store, scoped to one stack in its
/// region: every secret a `SecureString` at its [`SecretScope::path`]
/// (`/app/stage/label`), the note and roll its description
/// ([`SecretMeta::description`]), encrypted under the account's `aws/ssm`
/// key, which authorises account principals through its own key policy so a
/// reader needs no `kms:` grant. What `<SsmSecrets/>` (declared or
/// implicit) attaches.
///
/// A stack's secrets nest under one prefix ([`SecretScope::prefix`]), which
/// is what an instance role grants in one statement and what
/// [`list`](SecretStoreProvider::list) walks; another stack of the same
/// region is [`for_scope`](SecretStoreProvider::for_scope) away.
#[derive(Debug, Clone)]
pub struct SsmSecretStore {
	scope: SecretScope,
	client: AwsJsonClient,
}

impl SsmSecretStore {
	/// The provider id.
	pub const ID: &'static str = "ssm";

	/// The store over parameter store in `region`, scoped to `scope`.
	pub fn new(scope: SecretScope, region: impl Into<SmolStr>) -> Self {
		Self {
			scope,
			client: AwsJsonClient::new("ssm", "AmazonSSM", region),
		}
	}

	/// The same store calling through `client`: explicit credentials, or a
	/// test's stand-in for AWS.
	pub fn with_client(mut self, client: AwsJsonClient) -> Self {
		self.client = client;
		self
	}

	/// Read a parameter, decrypting a `SecureString`. `Ok(None)` when it does
	/// not exist; any other failure (no credentials, no permission) is an
	/// error rather than a silent mint of a second secret.
	async fn get_parameter(&self, name: &str) -> Result<Option<String>> {
		match self
			.client
			.call(
				"GetParameter",
				&json!({"Name": name, "WithDecryption": true}),
			)
			.await
		{
			Ok(body) => body["Parameter"]["Value"]
				.as_str()
				.map(String::from)
				.ok_or_else(|| {
					bevyhow!("ssm GetParameter answered {name} without a value")
				})?
				.xmap(Some)
				.xok(),
			Err(err) if AwsError::code(&err) == Some("ParameterNotFound") => {
				Ok(None)
			}
			Err(err) => Err(err),
		}
	}

	/// Write a `SecureString`, replacing an existing one when `overwrite`;
	/// the create conflict is answered as
	/// [`SecretStoreError::AlreadyExists`].
	async fn put_parameter(
		&self,
		name: &str,
		value: &str,
		meta: &SecretMeta,
		overwrite: bool,
	) -> Result {
		let mut body = json!({
			"Name": name,
			"Value": value,
			"Type": "SecureString",
			"Overwrite": overwrite,
		});
		let description = meta.description();
		if !description.is_empty() {
			body["Description"] = description.into();
		}
		// the one call that carries a secret, so its errors are redacted
		match self.client.call_secret("PutParameter", &body, value).await {
			Ok(_) => Ok(()),
			Err(err)
				if AwsError::code(&err) == Some("ParameterAlreadyExists") =>
			{
				Err(SecretStoreError::AlreadyExists {
					address: name.into(),
				}
				.into())
			}
			Err(err) => Err(err),
		}
	}

	/// Every parameter's metadata under `prefix`, recursively, without a
	/// value: `DescribeParameters` is the one call that reports the
	/// description. Names are filtered against `prefix/` as well as the
	/// api's own path filter, so a stage whose name is a prefix of another's
	/// (`drill`, `drill-two`) can never list its neighbour on an api subtlety.
	async fn describe_parameters(&self, prefix: &str) -> Result<Vec<Value>> {
		self.paginate(
			"DescribeParameters",
			json!({
				"ParameterFilters": [
					{"Key": "Path", "Option": "Recursive", "Values": [prefix]}
				],
				"MaxResults": 50,
			}),
		)
		.await?
		.xmap(|items| Self::under(prefix, items))
		.xok()
	}

	/// Every parameter under `prefix` with its value, decrypted.
	async fn get_parameters_by_path(&self, prefix: &str) -> Result<Vec<Value>> {
		self.paginate(
			"GetParametersByPath",
			json!({
				"Path": prefix,
				"Recursive": true,
				"WithDecryption": true,
				"MaxResults": 10,
			}),
		)
		.await?
		.xmap(|items| Self::under(prefix, items))
		.xok()
	}

	/// Every page's `Parameters` of `action`, following `NextToken`.
	async fn paginate(
		&self,
		action: &str,
		mut body: Value,
	) -> Result<Vec<Value>> {
		let mut items = Vec::new();
		loop {
			let mut page = self.client.call(action, &body).await?;
			if let Value::Array(parameters) = page["Parameters"].take() {
				items.extend(parameters);
			}
			match page["NextToken"].as_str() {
				Some(token) => body["NextToken"] = token.into(),
				None => return items.xok(),
			}
		}
	}

	/// Delete parameters by name, in the api's batches of ten, answering the
	/// names actually deleted: one already gone is reported under
	/// `InvalidParameters` and simply absent here.
	async fn delete_parameters(&self, names: &[String]) -> Result<Vec<String>> {
		let mut deleted = Vec::new();
		for batch in names.chunks(10) {
			let body = self
				.client
				.call("DeleteParameters", &json!({"Names": batch}))
				.await?;
			deleted.extend(
				body["DeletedParameters"]
					.as_array()
					.into_iter()
					.flatten()
					.filter_map(|name| name.as_str().map(String::from)),
			);
		}
		Ok(deleted)
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

	/// The label of a parameter under this store's prefix, ie the
	/// `db-password` of `/app/stage/db-password`.
	fn label_of(&self, name: &str) -> Option<SecretRef> {
		name.strip_prefix(&format!("{}/", self.scope.prefix()))
			.filter(|label| !label.is_empty())
			.map(SecretRef::new)
	}

	/// One listed parameter as an entry.
	fn entry(&self, item: &Value) -> Option<SecretEntry> {
		let name = item["Name"].as_str()?;
		let meta = SecretMeta::parse_description(
			item["Description"].as_str().unwrap_or_default(),
		);
		SecretEntry {
			secret: self.label_of(name)?,
			address: name.into(),
			note: meta.note,
			roll: meta.roll,
			// the api answers epoch seconds, ie `1757898061.5`
			modified: item["LastModifiedDate"].as_f64().map(|secs| {
				Timestamp::from_millis((secs * 1_000.0).round() as i64)
			}),
		}
		.xmap(Some)
	}
}

impl SecretStoreProvider for SsmSecretStore {
	fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { Self::ID }

	fn region(&self) -> Option<SmolStr> { Some(self.client.region().into()) }

	fn scope(&self) -> &SecretScope { &self.scope }

	/// The same region serves every stack, so a rescope keeps this one.
	fn for_scope(
		&self,
		scope: &SecretScope,
	) -> Result<Box<dyn SecretStoreProvider>> {
		let store: Box<dyn SecretStoreProvider> = Box::new(Self {
			scope: scope.clone(),
			client: self.client.clone(),
		});
		store.xok()
	}

	/// The parameter name, ie `/beetmash/prod/db-password`.
	fn address(&self, secret: &SecretRef) -> SmolStr {
		self.scope.path(secret).into()
	}

	fn get(
		&self,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let this = self.clone();
		Box::pin(
			async move { this.get_parameter(&this.scope.path(&secret)).await },
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
			this.put_parameter(&this.scope.path(&secret), &value, &meta, false)
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
			this.put_parameter(&this.scope.path(&secret), &value, &meta, true)
				.await
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
		let this = self.clone();
		Box::pin(async move {
			this.describe_parameters(&this.scope.prefix())
				.await?
				.iter()
				.filter_map(|item| this.entry(item))
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// The descriptions from one listing and the values from one decrypting
	/// walk, joined by name.
	fn read_all(&self) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.clone();
		Box::pin(async move {
			let prefix = this.scope.prefix();
			let entries = this.describe_parameters(&prefix).await?;
			let values = this.get_parameters_by_path(&prefix).await?;
			values
				.iter()
				.filter_map(|item| {
					let mut entry = this.entry(item)?;
					let meta = SecretMeta::parse_description(
						entries
							.iter()
							.find(|described| described["Name"] == item["Name"])
							.and_then(|described| {
								described["Description"].as_str()
							})
							.unwrap_or_default(),
					);
					(entry.note, entry.roll) = (meta.note, meta.roll);
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
				.map(|secret| this.scope.path(secret))
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

#[cfg(test)]
mod test {
	use super::*;
	#[cfg(feature = "vault")]
	use crate::types::test_support::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	fn stack() -> ResolvedStack {
		Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default())
			.with_aws_region("ap-southeast-2")
	}

	/// Every call a [`recording`] store made: its target and json body.
	type Calls = Arc<Mutex<Vec<(String, Value)>>>;

	/// The store for [`stack`] whose AWS is `answer`, given each call's
	/// action and body, recording every call; signs as a fixed pair, so no
	/// credential and no network is touched.
	fn recording(
		answer: impl 'static + Send + Sync + Fn(&str, &Value) -> Response,
	) -> (SecretStore, Calls) {
		let calls = Calls::default();
		let recorded = calls.clone();
		let answer = Arc::new(answer);
		let client = AwsJsonClient::new("ssm", "AmazonSSM", "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, answer) = (recorded.clone(), answer.clone());
				Box::pin(async move {
					let target = request
						.headers
						.first_raw("x-amz-target")
						.unwrap_or_default()
						.to_string();
					request
						.headers
						.first_raw("authorization")
						.unwrap()
						.xpect_contains("/ap-southeast-2/ssm/aws4_request");
					let body = serde_json::from_slice::<Value>(
						&request.body.into_bytes().await?,
					)?;
					let response = answer(&target, &body);
					recorded.lock().unwrap().push((target, body));
					response.xok()
				})
			});
		let stack = stack();
		let store = SsmSecretStore::new(
			stack.secret_scope(),
			stack.aws_region().unwrap().clone(),
		)
		.with_client(client);
		(SecretStore::new(store), calls)
	}

	/// A json answer.
	fn ok(body: Value) -> Response {
		Response::ok().with_body(body.to_string())
	}

	/// An error answer carrying `code`.
	fn error(code: &str) -> Response {
		Response::from_status(StatusCode::BAD_REQUEST).with_body(
			json!({"__type": format!("com.amazonaws.ssm#{code}"), "message": "no"})
				.to_string(),
		)
	}

	/// The composition the live grants already carry, so the strings are
	/// pinned: a renamed parameter is a box that boots without its secret.
	#[beet_core::test]
	fn addresses_are_the_parameter_names() {
		let (store, _) = recording(|_, _| ok(json!({})));
		store
			.address(&SecretRef::new("db-password"))
			.as_str()
			.xpect_eq("/beetmash/prod/db-password");
		store.region().unwrap().as_str().xpect_eq("ap-southeast-2");
		// a rescope keeps the region and composes the other stack's names
		let drill = Stack::new("beetmash")
			.with_stage("drill")
			.resolve(&PackageConfig::default());
		store
			.for_scope(&drill.secret_scope())
			.unwrap()
			.address(&SecretRef::new("db-password"))
			.as_str()
			.xpect_eq("/beetmash/drill/db-password");
	}

	/// A read is a decrypting `GetParameter` of the secret's path, and a
	/// missing parameter is absence rather than an error.
	#[beet_core::test]
	async fn reads_a_parameter() {
		let (store, calls) = recording(|_, body| match body["Name"].as_str() {
			Some("/beetmash/prod/ssh-host-key") => {
				ok(json!({"Parameter": {"Value": "the key"}}))
			}
			_ => error("ParameterNotFound"),
		});
		store
			.get(&SecretRef::new("ssh-host-key"))
			.await
			.unwrap()
			.xpect_eq(Some("the key".into()));
		store
			.get(&SecretRef::new("absent"))
			.await
			.unwrap()
			.xpect_none();
		calls.lock().unwrap()[0].clone().xpect_eq((
			"AmazonSSM.GetParameter".into(),
			json!({"Name": "/beetmash/prod/ssh-host-key", "WithDecryption": true}),
		));
	}

	/// A create is a `SecureString` put that never overwrites, its
	/// description the roll and note, and the api's conflict is the store's.
	#[beet_core::test]
	async fn writes_a_parameter() {
		let (store, calls) =
			recording(|_, body| match body["Overwrite"].as_bool() {
				Some(false) => error("ParameterAlreadyExists"),
				_ => ok(json!({"Version": 2})),
			});
		let secret = SecretRef::new("db-password");
		let err = store
			.create(&secret, "hunter2", Some("the db"), SecretRoll::Remint)
			.await
			.unwrap_err();
		SecretStoreError::is_already_exists(&err).xpect_true();
		err.to_string().xnot().xpect_contains("hunter2");
		store
			.overwrite(&secret, "hunter3", None, None)
			.await
			.unwrap();
		let calls = calls.lock().unwrap();
		calls[0].clone().xpect_eq((
			"AmazonSSM.PutParameter".into(),
			json!({
				"Name": "/beetmash/prod/db-password",
				"Value": "hunter2",
				"Type": "SecureString",
				"Overwrite": false,
				"Description": "remint :: the db",
			}),
		));
		// no metadata, no description: the stored one is kept
		calls[1].1["Description"].xpect_eq(Value::Null);
		calls[1].1["Overwrite"].xpect_eq(json!(true));
	}

	/// A listing follows every page under the stack's own prefix, reading
	/// labels, notes and dates back and never a neighbouring stage.
	#[beet_core::test]
	async fn lists_every_page() {
		let (store, calls) =
			recording(|_, body| match body["NextToken"].as_str() {
				None => ok(json!({
					"Parameters": [{
						"Name": "/beetmash/prod/dkim-example-com",
						"Description": "manual:a new selector :: the signing key",
						"LastModifiedDate": 1757898061.5,
					}],
					"NextToken": "page-2",
				})),
				Some(_) => ok(json!({
					"Parameters": [
						{"Name": "/beetmash/prod/db-password"},
						{"Name": "/beetmash/prod-two/x"},
					],
				})),
			});
		let entries = store.list().await.unwrap();
		entries.len().xpect_eq(2);
		entries[0]
			.secret
			.label()
			.as_str()
			.xpect_eq("dkim-example-com");
		entries[0]
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("the signing key");
		entries[0]
			.roll
			.clone()
			.xpect_eq(Some(SecretRoll::manual("a new selector")));
		entries[0]
			.modified
			.unwrap()
			.millis()
			.xpect_eq(1757898061500);
		entries[1].secret.label().as_str().xpect_eq("db-password");
		let calls = calls.lock().unwrap();
		calls.len().xpect_eq(2);
		calls[0].0.as_str().xpect_eq("AmazonSSM.DescribeParameters");
		calls[0].1["ParameterFilters"].xpect_eq(json!([
			{"Key": "Path", "Option": "Recursive", "Values": ["/beetmash/prod"]}
		]));
		calls[1].1["NextToken"].xpect_eq(json!("page-2"));
	}

	/// A delete answers what the api deleted, one already gone absent.
	#[beet_core::test]
	async fn deletes_parameters() {
		let (store, calls) = recording(|_, _| {
			ok(json!({
				"DeletedParameters": ["/beetmash/prod/a"],
				"InvalidParameters": ["/beetmash/prod/b"],
			}))
		});
		store
			.delete(&[SecretRef::new("a"), SecretRef::new("b")])
			.await
			.unwrap()
			.xpect_eq(vec![SecretRef::new("a")]);
		calls.lock().unwrap()[0].clone().xpect_eq((
			"AmazonSSM.DeleteParameters".into(),
			json!({"Names": ["/beetmash/prod/a", "/beetmash/prod/b"]}),
		));
	}

	/// With nothing declared the stack carries an implicit `<SsmSecrets/>`,
	/// parameter store in the stack's region whatever the launch, and a
	/// declared one is the same store.
	#[cfg(feature = "vault")]
	#[beet_core::test]
	fn an_undeclared_stack_is_ssm() {
		let mut world = infra_world();
		let root = stack_root(&mut world, ());
		world.with_state::<StackQuery, _>(|stacks| {
			let stack = stacks.resolve(root);
			let store = stacks.secret_store(root).unwrap();
			store.id().xpect_eq("ssm");
			store
				.region()
				.unwrap()
				.xpect_eq(stack.aws_region().unwrap().clone());
			store.scope().clone().xpect_eq(stack.secret_scope());
		});
		// (one repo store per world, so a second world)
		let mut world = infra_world();
		let declared = stack_root(&mut world, SsmSecrets);
		world.with_state::<StackQuery, _>(|stacks| {
			stacks.secret_store(declared).unwrap().id().xpect_eq("ssm");
		});
	}

	/// A store declared beneath a stack would serve nothing, so it is
	/// refused rather than silently passed over for the implicit one.
	#[cfg(feature = "vault")]
	#[beet_core::test]
	fn a_nested_declaration_is_refused() {
		let mut world = infra_world();
		let root = stack_root(&mut world, ());
		let nested = world.spawn(ChildOf(root)).id();
		world.with_state::<StackQuery, _>(|stacks| {
			stacks.refuse_nested_store(root, "SsmSecrets").unwrap();
			stacks
				.refuse_nested_store(nested, "SsmSecrets")
				.unwrap_err()
				.to_string()
				.xpect_contains("declared beneath the stack `app--prod`");
		});
	}
}
