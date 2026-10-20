//! A secret a deployed runtime reads from its stack's secret store.

use crate::prelude::*;
use beet_core::prelude::*;

/// `<SecretBlock bx:ref="ssh_host_key" label="ssh-host-key"
/// env="BEET_SSH_HOST_KEY"/>`: one secret of the stack that a running
/// process reads, declared under its `<Stack>` and outside every `bx:cfg`, so
/// the deployer and the deployed binary read the same declaration. Its value
/// is a record of the deploying entry's secrets document, loaded into the
/// launch's environment as `env`.
///
/// Both meanings hang off this one entity:
///
/// - **Deploy**: a tofu resource, the `SecureString` parameter at the stack's
///   secret path (`/beet-site/prod/ssh-host-key`), so the stack's state owns
///   its create, its update on a roll and its destroy. Its value is
///   WRITE-ONLY: `value_wo` reads an ephemeral, sensitive variable
///   ([`variable`](Self::variable)) that only an apply fills, from the
///   launch's `env` on the tofu child's environment, so the value is in no
///   rendered config, no plan and no state. `value_wo_version` is the
///   record's `modified` ([`version`](Self::version)), read off the
///   document's plaintext index, so rolling the record rewrites the
///   parameter and nothing about the value is published. A compute naming
///   this declaration through [`ReadsSecrets`] grants read on exactly its
///   address and depends on the parameter, so a rebuilt box boots after it.
/// - **Runtime**: an attach lands a [`StoredSecret`] here by the launch's
///   [`ServiceAccess`] ([`stored`](Self::stored)): the record `env` in the
///   launch's environment locally (what `secrets.toml` loaded, absent for a
///   contributor without the identity), `label` in the stack's secret store
///   when deployed, where absence is an error naming the deploy that writes
///   it. A consumer names this entity by relation, ie
///   `{SshHostKeyRef($ssh_host_key)}`.
///
/// So the value is in no rendered config: not a box's `user_data`, not
/// `main.tf.json`, not a plan, not the tofu state, and not a file on the box.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct SecretBlock {
	/// The secret's label in the stack's store, ie the `ssh-host-key` in
	/// `/beet-site/prod/ssh-host-key`.
	label: SmolStr,
	/// The launch's environment variable holding the value: the deployer's
	/// to write, a local launch's to read, and the name of the record whose
	/// `modified` versions the parameter.
	#[set_with(into)]
	env: SmolStr,
	/// What the secret is, stored beside the value; empty composes one
	/// naming the record it is written from.
	#[set_with(into)]
	note: SmolStr,
}

impl Default for SecretBlock {
	fn default() -> Self { Self::new("", "") }
}

impl SecretBlock {
	/// The tofu resource type the deploy meaning renders.
	pub const RESOURCE_TYPE: &'static str = "aws_ssm_parameter";

	/// The secret `label`, written from the launch's `env`.
	pub fn new(label: impl Into<SmolStr>, env: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			env: env.into(),
			note: default(),
		}
	}

	/// The secret this declares, as the stack's store names it.
	pub fn secret(&self) -> SecretRef { SecretRef::new(self.label.clone()) }

	/// The note stored beside the value: the declared one, else one naming
	/// the record it is written from.
	pub fn stored_note(&self) -> SmolStr {
		match self.note.is_empty() {
			true => format!(
				"`{}`, written from the deploying entry's secrets document",
				self.env
			)
			.into(),
			false => self.note.clone(),
		}
	}

	/// How the stored copy rolls: roll the record, then deploy.
	pub fn roll(&self) -> SecretRoll {
		SecretRoll::manual(format!(
			"beet secrets/set {} (the record of the deploying entry's secrets \
			document)\n> deploy each stage: the record's new `modified` moves \
			the parameter's `value_wo_version`, so the full apply rewrites it \
			and the release restarts the reader",
			self.env
		))
	}

	/// The ephemeral, sensitive input the parameter's `value_wo` reads, named
	/// for the record (`var.BEET_SSH_HOST_KEY`): empty for every verb that
	/// plans, the launch's `env` for an apply, which refuses without it.
	pub fn variable(&self) -> Variable {
		Variable::write_only(self.env.clone()).with_sensitive(true)
	}

	/// The tofu address of the parameter in `stack`, ie
	/// `aws_ssm_parameter.beet_site__prod__ssh_host_key`: what a compute
	/// reading it depends on.
	pub fn parameter_address(&self, stack: &ResolvedStack) -> String {
		format!(
			"{}.{}",
			Self::RESOURCE_TYPE,
			stack.resource_ident(self.label.clone()).label()
		)
	}

	/// The parameter's `value_wo_version`: the `modified` of the record
	/// `env`, in milliseconds since the epoch, from whichever loaded
	/// document's plaintext index holds it. Every `secrets/set` of a new value
	/// moves it, so a roll rewrites the parameter while a re-set of the same
	/// value (a new note) leaves it alone, and it says nothing about
	/// the value the index does not already publish. Read off the index, not
	/// the sealed side, so a launch whose identity cannot open the record
	/// still plans and validates. A record no document holds, or one two
	/// documents disagree on, is an error naming it.
	#[cfg(feature = "vault")]
	pub fn version<'a>(
		&self,
		documents: impl IntoIterator<Item = &'a SecretsDocument>,
	) -> Result<i64> {
		let modified = documents
			.into_iter()
			.filter_map(|document| document.record(&self.env))
			.map(|(_, record)| record.modified)
			.collect::<BTreeSet<_>>();
		match modified.into_iter().collect::<Vec<_>>().as_slice() {
			[Some(modified)] => modified.millis().xok(),
			[] => bevybail!(
				"the `{}` record is in no loaded secrets document, so the \
				stack secret `{}` has no version: `beet secrets/set {} \
				--role=env_var ..` seals it into the entry's document, which a \
				`<Secrets/>` declaration loads",
				self.env,
				self.label,
				self.env
			),
			[None] => bevybail!(
				"the `{}` record carries no `modified`, which versions the \
				stack secret `{}`: `beet secrets/set {} --from-env ..` \
				rewrites it with one",
				self.env,
				self.label,
				self.env
			),
			_ => bevybail!(
				"the `{}` record is in two loaded secrets documents that \
				disagree on its `modified`: a name is loaded from exactly one",
				self.env
			),
		}
	}

	/// The [`StoredSecret`] this declaration means under `access`: the
	/// record `env` in the launch's environment locally, absence legal, and
	/// `label` in the stack's `store` when deployed, required.
	pub fn stored(
		&self,
		access: ServiceAccess,
		store: impl FnOnce() -> Result<SecretStore>,
	) -> Result<StoredSecret> {
		match access {
			ServiceAccess::Local => StoredSecret::new(
				SecretStore::new(EnvSecretStore),
				SecretRef::new(self.env.clone()),
			),
			ServiceAccess::Remote => StoredSecret::new(store()?, self.secret())
				.required(format!(
					"deploy the stack, whose full apply writes the `{}` record \
					into it",
					self.env
				)),
		}
		.xok()
	}
}

impl Block for SecretBlock {
	fn label(&self) -> &SmolStr { &self.label }

	// no stack-wide grant: only a compute naming it in `ReadsSecrets` reads it

	/// The write-only input the parameter reads, which an apply fills.
	fn variables(&self, _stack: &ResolvedStack) -> Vec<Variable> {
		vec![self.variable()]
	}
}

/// The [`DeployRender`] render system and its emission, registered by
/// [`InfraPlugin`] beside [`declare`](crate::types::declare).
#[cfg(all(feature = "bindings_aws_ssm", feature = "vault"))]
impl SecretBlock {
	/// Render each declared secret as its stack's parameter, versioned by
	/// its record in the loaded secrets documents.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		stacks: StackQuery,
		blocks: Query<(Entity, &SecretBlock)>,
		documents: Query<&SecretsDocument>,
	) {
		for (entity, block) in blocks.iter() {
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			let stack = stacks.resolve(entity);
			let (_, config) = scope.ctx();
			if let Err(err) = stacks.secret_store(entity).and_then(|store| {
				block.emit(
					&stack,
					&store,
					block.version(documents.iter())?,
					config,
				)
			}) {
				scope.error(bevyhow!("SecretBlock '{}': {err}", block.label()));
			}
		}
	}

	/// Emit the parameter and declare the variable it reads. Refuses a stack
	/// whose secret store is not parameter store: the box would read a
	/// store the parameter is not in.
	fn emit(
		&self,
		stack: &ResolvedStack,
		store: &SecretStore,
		version: i64,
		config: &mut terra::Config,
	) -> Result {
		if store.id() != SsmSecretStore::ID {
			bevybail!(
				"its stack keeps its secrets in `{}`, but a `<SecretBlock/>` \
				is written as a parameter store parameter: declare \
				`{{SsmSecrets}}` on the stack, or none for the implicit one",
				store.describe()
			);
		}
		self.emit_parameter(stack, version, config)
	}

	/// Declare the variable and add the parameter reading it.
	fn emit_parameter(
		&self,
		stack: &ResolvedStack,
		version: i64,
		config: &mut terra::Config,
	) -> Result {
		let variable = self.variable();
		config.ensure_variable(
			variable.key().as_str(),
			variable.tf_declaration(),
		);
		config.add_resource(&self.parameter(stack, version))?;
		Ok(())
	}

	/// The parameter itself, see the type docs.
	fn parameter(
		&self,
		stack: &ResolvedStack,
		version: i64,
	) -> terra::ResourceDef<crate::bindings::AwsSsmParameterDetails> {
		terra::ResourceDef::new_secondary(
			stack.resource_ident(self.label.clone()),
			crate::bindings::AwsSsmParameterDetails {
				name: stack.secret_path(&self.secret()).into(),
				r#type: "SecureString".into(),
				description: Some(
					SecretMeta::new(
						Some(&self.stored_note()),
						Some(self.roll()),
					)
					.description()
					.into(),
				),
				value_wo: Some(self.variable().tf_var_ref().into()),
				value_wo_version: Some(version),
				tags: Some(stack.tags()),
				..default()
			},
		)
	}
}

#[cfg(all(test, feature = "vault"))]
impl SecretBlock {
	/// A document index holding this block's record, written at `modified`
	/// milliseconds: what a render versions the parameter from.
	pub(crate) fn test_index(&self, modified: i64) -> SecretsDocument {
		let mut document = SecretsDocument::default();
		document.groups.insert("default".into(), SecretsGroup {
			recipients: Vec::new(),
			secrets: [(self.env.clone(), SecretRecord {
				role: Some(SecretRole::EnvVar),
				modified: Some(Timestamp::from_millis(modified)),
				..default()
			})]
			.into_iter()
			.collect(),
		});
		document
	}
}

/// Observer: land the [`StoredSecret`] a `<SecretBlock/>` means for this
/// launch's [`ServiceAccess`] on its entity. Deferred through the command
/// queue like every attach, so the stack's store declaration, on the stack or
/// an ancestor, has landed when it resolves.
pub(crate) fn attach_secret_block(
	ev: On<Insert<SecretBlock>>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let block = entity.get_or_else::<SecretBlock>()?.clone();
			let stored =
				block.stored(BootstrapConfig::get().service_access, || {
					entity.with_state::<StackQuery, _>(|entity, stacks| {
						stacks.secret_store(entity)
					})
				})?;
			entity.insert(stored);
			Ok(())
		});
}

/// The secrets a compute's runtime reads from its stack's secret store, each
/// a [`SecretBlock`] named by `bx:ref`:
/// `<LightsailBeetSiteBlock {ReadsSecrets([$ssh_host_key])}/>`. The compute
/// grants read on exactly those addresses and no other compute in the stack
/// is granted them, which is why a `<SecretBlock/>` declares no stack-wide
/// grant. Lowered by `LightsailBlock`, whose IAM user policy it extends;
/// another compute refuses it at render until it lowers the grant too.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct ReadsSecrets(#[entities] pub Vec<Entity>);

/// The secrets a compute reads, through its [`ReadsSecrets`].
#[derive(SystemParam)]
pub struct ReadsSecretsQuery<'w, 's> {
	reads: Query<'w, 's, &'static ReadsSecrets>,
	blocks: Query<'w, 's, &'static SecretBlock>,
	stacks: StackQuery<'w, 's>,
}

impl ReadsSecretsQuery<'_, '_> {
	/// Refuse a `compute` that names secrets but cannot grant them: loud at
	/// render, rather than a runtime denied its first read.
	pub fn refuse(&self, compute: Entity) -> Result {
		match self.reads.get(compute) {
			Ok(reads) if !reads.0.is_empty() => bevybail!(
				"this compute names secrets through `ReadsSecrets`, which only \
				a `LightsailBlock` grants: no other compute lowers the read \
				into its identity's policy yet"
			),
			_ => Ok(()),
		}
	}

	/// The secrets `compute` reads, each a [`SecretBlock`] under its own
	/// stack: what it grants read on ([`SecretBlock::secret`]) and depends
	/// on ([`SecretBlock::parameter_address`]). A target that is no
	/// `<SecretBlock/>`, or one declared under another stack (whose store
	/// this compute's grants do not reach), is an error naming the relation.
	pub fn of(&self, compute: Entity) -> Result<Vec<SecretBlock>> {
		let Ok(reads) = self.reads.get(compute) else {
			return Vec::new().xok();
		};
		let root = self.stacks.root(compute).map(|(root, _)| root).ok();
		reads
			.0
			.iter()
			.map(|target| {
				let block = self.blocks.get(*target).map_err(|_| {
					bevyhow!(
						"`ReadsSecrets` names entity {target}, which is no \
						`<SecretBlock/>`"
					)
				})?;
				if self.stacks.root(*target).map(|(root, _)| root).ok() != root
				{
					bevybail!(
						"`ReadsSecrets` names the secret `{}`, declared under \
						another stack: a compute reads its own stack's secrets",
						block.label()
					);
				}
				block.clone().xok()
			})
			.collect()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::types::test_support::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A stack declaring the host key secret over a document store, with the
	/// record already written when `stored` is set, and a server naming the
	/// secret's declaration; answers the server entity. Native, like the ssh
	/// server it stands in for: a wasm wait is not `Send`.
	#[cfg(not(target_arch = "wasm32"))]
	async fn server_reading(world: &mut World, stored: Option<&str>) -> Entity {
		let root = stack_root(world, DocumentSecrets::new("secrets.toml"));
		let block = SecretBlock::new("ssh-host-key", "BEET_TEST_HOST_KEY");
		let declaration = world.spawn((block.clone(), ChildOf(root))).id();
		world.flush();
		let store = world
			.with_state::<StackQuery, _>(|stacks| stacks.secret_store(root))
			.unwrap();
		if let Some(value) = stored {
			store
				.create(&block.secret(), value, None, block.roll())
				.await
				.unwrap();
		}
		// the deployed meaning, whatever this test process's own access
		let stored =
			block.stored(ServiceAccess::Remote, || store.xok()).unwrap();
		world.entity_mut(declaration).insert(stored);
		world.spawn(SshHostKeyRef(declaration)).id()
	}

	/// What `server` resolves as its host key, encoded.
	#[cfg(not(target_arch = "wasm32"))]
	async fn resolved(world: &mut World, server: Entity) -> Result<String> {
		world
			.entity_mut(server)
			.run_async_then(async |entity| -> Result<String> {
				format!("{:?}", SshHostKey::resolve(&entity).await?).xok()
			})
			.await
	}

	/// A deployed ssh server reads its key from the stack's secret store, and
	/// a key missing from the store fails the server naming its address and
	/// the deploy that writes it, never a fall back to a key nobody pinned.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn the_server_reads_its_key_from_the_store() {
		let mut world = infra_world();
		// a real key: a build linking the ssh server parses what it reads
		let server = server_reading(&mut world, Some(SshHostKey::DEBUG)).await;
		resolved(&mut world, server)
			.await
			.unwrap()
			// the key itself is redacted; presence is the answer
			.xpect_eq("Some(SshHostKey(..))");

		let mut world = infra_world();
		let server = server_reading(&mut world, None).await;
		resolved(&mut world, server)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no secret at")
			.xpect_contains("ssh-host-key")
			.xpect_contains("BEET_TEST_HOST_KEY");
	}

	/// A local launch's declaration reads the record its `secrets.toml`
	/// loads, by the record's name in the launch's environment, and never
	/// resolves the stack's store.
	#[beet_core::test]
	fn a_local_launch_reads_its_record() {
		let mut world = infra_world();
		let root = stack_root(&mut world, ());
		let declaration = world
			.spawn((
				SecretBlock::new("ssh-host-key", "BEET_TEST_LOCAL_KEY"),
				ChildOf(root),
			))
			.id();
		world.flush();
		let stored = world.entity(declaration).get::<StoredSecret>().unwrap();
		stored.store().id().xpect_eq("env");
		stored
			.secret()
			.label()
			.as_str()
			.xpect_eq("BEET_TEST_LOCAL_KEY");
		stored.supplied_by().xpect_none();
	}

	/// The deploy meaning is one `SecureString` parameter at the stack's
	/// secret path, its value WRITE-ONLY: `value_wo` reads an ephemeral,
	/// sensitive variable with no default, never a literal, and no `value`
	/// is rendered at all. Its version is the record's `modified`, it carries
	/// the stack's tags, and the config pins the tofu that reads it.
	#[cfg(all(feature = "bindings_aws_ssm", feature = "vault"))]
	#[beet_core::test]
	fn renders_a_write_only_parameter() {
		let block = SecretBlock::new("ssh-host-key", "BEET_TEST_WRITE_ONLY");
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(block.test_index(1_760_000_000_000));
			parent.spawn(block.clone());
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		let json = serde_json::from_str::<serde_json::Value>(
			&config.to_json_string().unwrap(),
		)
		.unwrap();
		let parameters = json["resource"][SecretBlock::RESOURCE_TYPE]
			.as_object()
			.unwrap();
		parameters.len().xpect_eq(1);
		let (label, body) = parameters.iter().next().unwrap();
		format!("{}.{label}", SecretBlock::RESOURCE_TYPE)
			.xpect_eq(block.parameter_address(&stack));
		body["name"]
			.as_str()
			.unwrap()
			.xpect_eq(stack.secret_path(&block.secret()));
		body["type"].as_str().unwrap().xpect_eq("SecureString");
		body["value_wo"]
			.as_str()
			.unwrap()
			.xpect_eq("${var.BEET_TEST_WRITE_ONLY}");
		body["value_wo_version"]
			.as_i64()
			.unwrap()
			.xpect_eq(1_760_000_000_000);
		body.get("value").xpect_none();
		body.get("insecure_value").xpect_none();
		body["tags"]["Project"]
			.as_str()
			.unwrap()
			.xpect_eq(stack.app_name().as_str());
		body["tags"]["Stage"]
			.as_str()
			.unwrap()
			.xpect_eq(stack.stage().as_str());
		body["description"]
			.as_str()
			.unwrap()
			.xpect_contains("manual:beet secrets/set BEET_TEST_WRITE_ONLY");
		// the input: ephemeral, sensitive, and nothing a bare apply could
		// write in its place
		let variable = &json["variable"]["BEET_TEST_WRITE_ONLY"];
		variable["ephemeral"].as_bool().unwrap().xpect_true();
		variable["sensitive"].as_bool().unwrap().xpect_true();
		variable.get("default").xpect_none();
		json["terraform"]["required_version"]
			.as_str()
			.unwrap()
			.xpect_eq(">= 1.11.0");
	}

	/// The version is the record's `modified`, so rolling the record moves
	/// it; a record no loaded document holds, one with no `modified` and one
	/// two documents disagree on are each refused naming it, while the same
	/// file loaded twice agrees with itself.
	#[cfg(all(feature = "bindings_aws_ssm", feature = "vault"))]
	#[beet_core::test]
	fn the_version_follows_the_record() {
		let block = SecretBlock::new("ssh-host-key", "BEET_TEST_VERSIONED");
		let first = block.test_index(1_000);
		let rolled = block.test_index(2_000);
		block.version([&first]).unwrap().xpect_eq(1_000);
		block.version([&rolled]).unwrap().xpect_eq(2_000);
		block
			.version([&first, &first.clone()])
			.unwrap()
			.xpect_eq(1_000);
		block
			.version([])
			.unwrap_err()
			.to_string()
			.xpect_contains("`BEET_TEST_VERSIONED` record is in no loaded");
		block
			.version([&first, &rolled])
			.unwrap_err()
			.to_string()
			.xpect_contains("disagree");
		let mut unstamped = first.clone();
		for group in unstamped.groups.values_mut() {
			for record in group.secrets.values_mut() {
				record.modified = None;
			}
		}
		block
			.version([&unstamped])
			.unwrap_err()
			.to_string()
			.xpect_contains("carries no `modified`");
	}

	/// A parameter is only read where the box reads, so a stack keeping its
	/// secrets in any other store is refused at render.
	#[cfg(all(feature = "bindings_aws_ssm", feature = "vault"))]
	#[beet_core::test]
	fn refuses_a_store_other_than_parameter_store() {
		let stack = Stack::new("app").resolve(&PackageConfig::default());
		SecretBlock::new("ssh-host-key", "BEET_TEST_ELSEWHERE")
			.emit(
				&stack,
				&memory_secret_store(&stack),
				1,
				&mut terra::Config::default(),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("a `<SecretBlock/>` is written as a parameter");
	}

	/// A fake parameter store on loopback answering the calls tofu's aws
	/// provider makes for one parameter, one connection per request. Logs
	/// each call's target, and for a write whether it carried `canary`, never
	/// the value itself.
	#[cfg(all(
		not(target_arch = "wasm32"),
		feature = "bindings_aws_ssm",
		feature = "vault"
	))]
	fn fake_ssm(
		canary: &'static str,
	) -> (String, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
		use serde_json::Value;
		use serde_json::json;
		use std::io::BufRead;
		use std::io::Read;
		use std::io::Write;
		let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
		let url = format!("http://{}", listener.local_addr().unwrap());
		let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
		let log = calls.clone();
		std::thread::spawn(move || {
			let mut params = HashMap::<String, Value>::default();
			let mut tags = HashMap::<String, Vec<Value>>::default();
			let mut handle = |target: &str, req: Value| -> (u16, Value) {
				let name = req["Name"].as_str().unwrap_or_default().to_string();
				let mut call = format!("{target} {name}");
				if let Some(value) = req["Value"].as_str() {
					let overwrite = req["Overwrite"].as_bool().unwrap_or(false);
					call += &format!(
						" overwrite={overwrite} canary={}",
						value == canary
					);
				}
				log.lock().unwrap().push(call);
				let missing = (
					400,
					json!({"__type": "ParameterNotFound", "message": ""}),
				);
				match target {
					"PutParameter" => {
						let version = params.get(&name).map_or(0, |param| {
							param["Version"].as_i64().unwrap()
						}) + 1;
						params.insert(
							name.clone(),
							json!({
								"Name": name, "Type": req["Type"], "Version": version,
								"Description": req["Description"], "Tier": "Standard",
								"DataType": "text", "LastModifiedDate": 0,
								"ARN": format!("arn:aws:ssm:us-west-2:000000000000:parameter{name}"),
							}),
						);
						if let Some(created) = req["Tags"].as_array() {
							tags.insert(name, created.clone());
						}
						(200, json!({"Version": version, "Tier": "Standard"}))
					}
					"GetParameter" => match params.get(&name) {
						// a stand-in: tofu must keep whatever it reads out of state
						Some(param) => {
							let mut param = param.clone();
							param["Value"] = json!("read-back");
							(200, json!({ "Parameter": param }))
						}
						None => missing,
					},
					"DescribeParameters" => (
						200,
						json!({ "Parameters": params.values().collect::<Vec<_>>() }),
					),
					"ListTagsForResource" => (
						200,
						json!({ "TagList": tags.get(req["ResourceId"].as_str().unwrap_or_default()) }),
					),
					"AddTagsToResource" => {
						let resource =
							req["ResourceId"].as_str().unwrap_or_default();
						tags.entry(resource.to_string()).or_default().extend(
							req["Tags"].as_array().cloned().unwrap_or_default(),
						);
						(200, json!({}))
					}
					"DeleteParameter" => match params.remove(&name) {
						Some(_) => (200, json!({})),
						None => missing,
					},
					_ => (200, json!({})),
				}
			};
			for mut stream in listener.incoming().flatten() {
				let mut reader =
					std::io::BufReader::new(stream.try_clone().unwrap());
				let (mut target, mut length) = (String::new(), 0);
				loop {
					let mut line = String::new();
					reader.read_line(&mut line).unwrap();
					let line = line.trim_end().to_ascii_lowercase();
					if line.is_empty() {
						break;
					}
					if let Some(value) = line.strip_prefix("content-length:") {
						length = value.trim().parse().unwrap();
					}
					if let Some(value) = line.strip_prefix("x-amz-target:") {
						target = value.trim().replace("amazonssm.", "");
					}
				}
				let mut body = vec![0; length];
				reader.read_exact(&mut body).unwrap();
				// the targets are matched in their documented case
				let target = [
					"PutParameter",
					"GetParameter",
					"DescribeParameters",
					"ListTagsForResource",
					"AddTagsToResource",
					"RemoveTagsFromResource",
					"DeleteParameter",
				]
				.into_iter()
				.find(|known| known.to_ascii_lowercase() == target)
				.unwrap_or("Unknown");
				let (status, reply) = handle(
					target,
					serde_json::from_slice(&body).unwrap_or_default(),
				);
				let reply = reply.to_string();
				write!(
					stream,
					"HTTP/1.1 {status} OK\r\nContent-Type: \
					application/x-amz-json-1.1\r\nContent-Length: {}\r\n\
					Connection: close\r\n\r\n{reply}",
					reply.len()
				)
				.unwrap();
			}
		});
		(url, calls)
	}

	/// tofu owns the parameter's whole lifecycle with the value nowhere it
	/// writes down: the apply creates it carrying the launch's value, a plan
	/// handed the empty placeholder every planning verb passes proposes
	/// nothing, a rolled record's version rewrites it, the state never holds
	/// the value, and a destroy plans and makes its deletion after the box
	/// that depends on it. Drives the real tofu and aws provider against a
	/// fake parameter store on loopback, the provider read from the plugin
	/// cache, so nothing leaves the machine.
	#[cfg(all(
		not(target_arch = "wasm32"),
		feature = "bindings_aws_ssm",
		feature = "vault"
	))]
	#[beet_core::test(timeout_ms = 120_000)]
	async fn tofu_owns_the_lifecycle() {
		const CANARY: &str = "write-only-canary-value";
		let (url, calls) = fake_ssm(CANARY);
		let block = SecretBlock::new("ssh-host-key", "BEET_TEST_TOFU_KEY");
		let stack = Stack::new("app")
			.with_stage("dev")
			.resolve(&PackageConfig::default());
		let dir = TempDir::new().unwrap();
		let write = |version: i64| {
			let mut config = terra::Config::default();
			block.emit_parameter(&stack, version, &mut config).unwrap();
			config
				.add_untyped_provider(
					"aws",
					&serde_json::json!({
						"region": "us-west-2",
						"access_key": "AKIAFAKEFAKEFAKEFAKE",
						"secret_key": "fake",
						"skip_credentials_validation": true,
						"skip_requesting_account_id": true,
						"skip_metadata_api_check": true,
						"endpoints": { "ssm": url },
					}),
				)
				.unwrap()
				.add_untyped_resource(
					"terraform_data",
					"box",
					&serde_json::json!({
						"input": "box",
						"depends_on": [block.parameter_address(&stack)],
					}),
				)
				.unwrap();
			fs_ext::write(
				dir.join("main.tf.json"),
				config.to_json_string().unwrap(),
			)
			.unwrap();
		};
		// tofu's output without its colors
		let plain = |output: String| {
			let mut text = String::new();
			let mut chars = output.chars();
			while let Some(char) = chars.next() {
				match char {
					'\x1b' => {
						chars.by_ref().find(|char| *char == 'm');
					}
					char => text.push(char),
				}
			}
			text
		};
		let valued = [terra::TofuVar::secret(block.env().clone(), CANARY)];
		let empty = [terra::TofuVar::secret(block.env().clone(), "")];
		write(1);
		terra::tofu::test_support::init_offline(&dir, &[])
			.await
			.unwrap();
		terra::tofu::apply(&dir, &valued).await.unwrap();
		plain(terra::tofu::plan(&dir, &empty).await.unwrap())
			.xpect_contains("No changes.");
		write(2);
		plain(terra::tofu::apply(&dir, &valued).await.unwrap())
			.xpect_contains("0 added, 1 changed, 0 destroyed");
		fs_ext::read_to_string(dir.join("terraform.tfstate"))
			.unwrap()
			.xnot()
			.xpect_contains(CANARY)
			.xnot()
			.xpect_contains("read-back");
		let address = block.parameter_address(&stack);
		let destroyed =
			plain(terra::tofu::destroy(&dir, &empty).await.unwrap());
		destroyed
			.as_str()
			.xpect_contains(&format!("# {address} will be destroyed"))
			.xpect_contains("Plan: 0 to add, 0 to change, 2 to destroy.");
		destroyed
			.find("terraform_data.box: Destroying")
			.unwrap()
			.xpect_less_than(
				destroyed.find(&format!("{address}: Destroying")).unwrap(),
			);
		let calls = calls.lock().unwrap().join("\n");
		calls
			.as_str()
			.xpect_contains(
				"PutParameter /app/dev/ssh-host-key overwrite=false canary=true",
			)
			.xpect_contains(
				"PutParameter /app/dev/ssh-host-key overwrite=true canary=true",
			)
			.xpect_contains("DeleteParameter /app/dev/ssh-host-key")
			.xnot()
			.xpect_contains("canary=false");
	}
}
