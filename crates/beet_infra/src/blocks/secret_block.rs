//! A secret a deployed runtime reads from its stack's secret store.

use crate::prelude::*;
use beet_core::prelude::*;

/// `<SecretBlock bx:ref="ssh_host_key" label="ssh-host-key"
/// env="BEET_SSH_HOST_KEY"/>` — one secret of the stack that a running
/// process reads, declared under its `<Stack>` and outside every `bx:cfg`, so
/// the deployer and the deployed binary read the same declaration. Its value
/// is a record of the deploying entry's secrets document, loaded into the
/// launch's environment as `env`.
///
/// Both meanings hang off this one entity:
///
/// - **Deploy**: the full `TofuApply` mirrors the launch's `env` into the
///   stack's [`SecretStore`] under `label` ([`mirror`](Self::mirror)), outside
///   tofu, before the apply boots anything that reads it. A compute naming
///   this declaration through [`ReadsSecrets`] grants read on exactly its
///   address, and nothing else in the stack is granted it.
/// - **Runtime**: an attach lands a [`SecretSource`] here by the launch's
///   [`ServiceAccess`]: the launch's own `env` locally (what `secrets.toml`
///   loaded, absent for a contributor without the identity), the stack's
///   secret store when deployed, where absence is an error naming the deploy
///   that mirrors it. A consumer names this entity by relation, ie
///   `{SshHostKeyRef($ssh_host_key)}`.
///
/// So the value is in no rendered config: not a box's `user_data`, not
/// `main.tf.json`, not the tofu state, and not a file on the box.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct SecretBlock {
	/// The secret's label in the stack's store, ie the `ssh-host-key` in
	/// `/beet-site/prod/ssh-host-key`.
	label: SmolStr,
	/// The launch's environment variable holding the value: the deployer's to
	/// mirror, and a local launch's to read.
	#[set_with(into)]
	env: SmolStr,
	/// What the secret is, stored beside the value; empty composes one
	/// naming the record it mirrors.
	#[set_with(into)]
	note: SmolStr,
}

impl Default for SecretBlock {
	fn default() -> Self { Self::new("", "") }
}

impl SecretBlock {
	/// The secret `label`, mirrored from the launch's `env`.
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
	/// the record it mirrors.
	pub fn stored_note(&self) -> SmolStr {
		match self.note.is_empty() {
			true => format!(
				"`{}`, mirrored from the deploying entry's secrets document",
				self.env
			)
			.into(),
			false => self.note.clone(),
		}
	}

	/// How the stored copy rolls: roll the record it mirrors, then deploy.
	pub fn roll(&self) -> SecretRoll {
		SecretRoll::manual(format!(
			"roll the `{}` record of the deploying entry's secrets document \
			(beet secrets/set {})\n> deploy each stage: its full apply mirrors \
			the record here and the release restarts the reader",
			self.env, self.env
		))
	}

	/// Write the launch's `env` into `store`, answering whether it wrote: an
	/// unchanged value and metadata write nothing. A launch without the
	/// record is an error naming it, since the box would boot without it.
	pub async fn mirror(&self, store: &SecretStore) -> Result<bool> {
		let value = env_ext::var(&self.env).map_err(|_| {
			bevyhow!(
				"`{}` is not in this launch's environment, so the stack secret \
				`{}` cannot be mirrored: the deploying entry's secrets document \
				holds it as an `env_var` record (`beet secrets/check` says \
				whether this machine's identity opens it)",
				self.env,
				self.label
			)
		})?;
		store
			.converge(
				&self.secret(),
				&value,
				Some(&self.stored_note()),
				Some(self.roll()),
			)
			.await
	}

	/// The [`SecretSource`] this declaration means under `access`: the
	/// launch's `env` locally, the stack's store, resolved from
	/// `declaration` when read, when deployed.
	pub fn source(
		&self,
		access: ServiceAccess,
		declaration: AsyncEntity,
	) -> SecretSource {
		match access {
			ServiceAccess::Local => SecretSource::env(self.env.clone()),
			ServiceAccess::Remote => SecretSource::new(StackSecretSource {
				declaration,
				secret: self.secret(),
				env: self.env.clone(),
			}),
		}
	}
}

impl Block for SecretBlock {
	fn label(&self) -> &SmolStr { &self.label }
	// no stack-wide grant: only a compute naming it in `ReadsSecrets` reads it
}

/// Observer: land the [`SecretSource`] a `<SecretBlock/>` means for this
/// launch's [`ServiceAccess`] on its entity. Deferred through the command
/// queue like every attach; the store itself resolves when the source is read,
/// so a store declaration landing after this one is still the one read.
pub(crate) fn attach_secret_block(
	ev: On<Insert<SecretBlock>>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let block = entity.get_or_else::<SecretBlock>()?.clone();
			let declaration = entity
				.world_scope(|world| {
					world.get_resource::<AsyncWorld>().cloned()
				})
				.ok_or_else(|| {
					bevyhow!(
						"`<SecretBlock/>` needs the `AsyncWorld` resource, which \
						`AsyncPlugin` inserts"
					)
				})?
				.entity(entity.id());
			entity.insert(
				block
					.source(BootstrapConfig::get().service_access, declaration),
			);
			Ok(())
		});
}

/// The deployed meaning of a [`SecretBlock`]: its value in the stack's
/// [`SecretStore`], resolved from the declaration when read, and required.
#[derive(Clone)]
struct StackSecretSource {
	declaration: AsyncEntity,
	secret: SecretRef,
	env: SmolStr,
}

impl SecretSourceProvider for StackSecretSource {
	fn box_clone(&self) -> Box<dyn SecretSourceProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { "stack" }

	fn describe(&self) -> String {
		format!("the stack secret `{}`", self.secret.label())
	}

	fn get(&self) -> SendBoxedFuture<Result<Option<String>>> {
		let this = self.clone();
		Box::pin(async move {
			let store = this
				.declaration
				.with_state::<StackQuery, _>(|entity, stacks| {
					stacks.secret_store(entity)
				})
				.await??;
			store
				.require(&this.secret, || {
					format!(
						"deploy the stack, whose full apply mirrors the `{}` \
						record into it",
						this.env
					)
				})
				.await?
				.xmap(Some)
				.xok()
		})
	}
}

/// The secrets a compute's runtime reads from its stack's secret store, each
/// a [`SecretBlock`] named by `bx:ref`:
/// `<LightsailBeetSiteBlock {ReadsSecrets([$ssh_host_key])}/>`. The compute
/// grants read on exactly those addresses and no other compute in the stack
/// is granted them, which is why a `<SecretBlock/>` declares no stack-wide
/// grant. Lowered by `LightsailBlock`, whose box carries the `aws` cli
/// parameter store is read through; another compute refuses it at render.
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
				a `LightsailBlock` grants: its box carries the `aws` cli a \
				stack secret is read through"
			),
			_ => Ok(()),
		}
	}

	/// The secrets `compute` reads, each a [`SecretBlock`] under its own
	/// stack. A target that is no `<SecretBlock/>`, or one declared under
	/// another stack (whose store this compute's grants do not reach), is an
	/// error naming the relation.
	pub fn of(&self, compute: Entity) -> Result<Vec<SecretRef>> {
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
				block.secret().xok()
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
	/// record already mirrored when `stored` is set, and a server naming the
	/// secret's declaration; answers the server entity. Native, like the ssh
	/// server it stands in for: a wasm wait is not `Send`.
	#[cfg(not(target_arch = "wasm32"))]
	async fn server_reading(world: &mut World, stored: Option<&str>) -> Entity {
		let root = world
			.spawn((
				Stack::new("app").with_stage("dev"),
				BlobStore::temp(),
				RepoStore,
				children![DocumentSecrets::new("secrets.toml")],
			))
			.id();
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
		let source = world.run_async_then(async move |world| {
			block.source(ServiceAccess::Remote, world.entity(declaration))
		});
		let source = source.await;
		world.entity_mut(declaration).insert(source);
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
	/// the deploy that mirrors it, never a fall back to a key nobody pinned.
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

	/// The mirror writes the launch's record into the store once, then
	/// writes nothing while it is unchanged; a launch without the record is
	/// an error naming it.
	#[beet_core::test]
	async fn mirrors_the_launch_record() {
		let stack = Stack::new("app").resolve(&PackageConfig::default());
		let store = memory_secret_store(&stack);
		let block = SecretBlock::new("mirror-test", "BEET_TEST_MIRRORED");
		block
			.mirror(&store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`BEET_TEST_MIRRORED` is not in this launch");
		// SAFETY: test-only, a variable no other test reads
		unsafe { env_ext::set_var("BEET_TEST_MIRRORED", "value") }.unwrap();
		block.mirror(&store).await.unwrap().xpect_true();
		block.mirror(&store).await.unwrap().xpect_false();
		store
			.get(&block.secret())
			.await
			.unwrap()
			.unwrap()
			.xpect_eq("value");
		store
			.meta(&block.secret())
			.await
			.unwrap()
			.unwrap()
			.roll
			.unwrap()
			.kind()
			.xpect_eq("manual");
		// SAFETY: as above
		unsafe { env_ext::remove_var("BEET_TEST_MIRRORED") }.unwrap();
	}
}
