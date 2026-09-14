//! A block whose resource is a blob store, and the erased half every consumer
//! of "the store, whichever kind" reads.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A [`Block`] whose resource is a blob store: a bucket the deploy creates, or
/// a [`StoreUriBlock`] naming one it does not. What every store block shares
/// is where a process reads it, a [`StoreUri`], and whether each deploy
/// publishes under its own id; [`ErasedStoreBlock`] projects exactly that, so
/// the consumers that want "the store" rather than "the bucket" (the repo
/// store a compute boots from, the destination a sync publishes to, the
/// ledger) never name a provider.
pub trait StoreBlock: Block {
	/// The uri of this store's ROOT resolved against `stack`: a bucket's
	/// composed name and region, a filesystem store's path. No per-deploy
	/// prefix, which [`ErasedStoreBlock::store_uri`] applies.
	fn store_uri(&self, stack: &ResolvedStack) -> StoreUri;

	/// Whether every deploy publishes under its own id below the root, so a
	/// process reads the document version it shipped with: the window between
	/// publishing a new document and swapping the binary that serves it is then
	/// not a window where the old binary parses the new document.
	fn deploy_versioned(&self) -> bool { false }
}

/// The erased half of any [`StoreBlock`], beside its [`ErasedBlock`]: the root
/// uri resolved against the declaration's stack, the local stand-in for the
/// same declaration, and the versioning flag, inserted by
/// [`on_insert`](Self::on_insert) and removed with the block.
///
/// The ROOT rather than a per-deploy uri, because the deploy id is a property
/// of the launch and may change within it (`<AdoptCurrentDeploy/>` points a
/// content sync at the live version): a consumer applies the id it holds
/// through [`store_uri`](Self::store_uri). Both uris are total, so the store
/// a process attaches is nothing but [`runtime_uri`](Self::runtime_uri) built.
#[derive(Debug, Clone, PartialEq, Get, Component)]
#[component(immutable)]
pub struct ErasedStoreBlock {
	/// The store's root, see [`StoreBlock::store_uri`].
	root: StoreUri,
	/// The local stand-in for the same declaration
	/// ([`ServiceAccess::local_store_uri`]), keyed by the composed resource
	/// name so it is exactly as distinct per app and stage as the root.
	local: StoreUri,
	/// See [`StoreBlock::deploy_versioned`].
	deploy_versioned: bool,
}

impl ErasedStoreBlock {
	/// The projection of `block` declared under `stack`.
	pub fn new(block: &impl StoreBlock, stack: &ResolvedStack) -> Self {
		Self {
			root: block.store_uri(stack),
			local: ServiceAccess::local_store_uri(
				&stack.resource_name(block.label().clone()),
			),
			deploy_versioned: block.deploy_versioned(),
		}
	}

	/// The uri a deploy hands a process: the root, nested under `deploy_id`
	/// when the store is versioned. The ONE place that uri is shaped, so the
	/// argv a lambda bakes, the env a release pointer publishes and the prefix a
	/// sync writes to cannot describe the same store differently.
	pub fn store_uri(&self, deploy_id: Option<&Uuid>) -> StoreUri {
		match (self.deploy_versioned, deploy_id) {
			(true, Some(deploy_id)) => {
				self.root.with_subdir(deploy_id.to_string())
			}
			_ => self.root.clone(),
		}
	}

	/// The store a process running under `access` attaches for this
	/// declaration: [`Remote`](ServiceAccess::Remote) reads
	/// [`store_uri`](Self::store_uri), exactly the uri a deploy bakes;
	/// [`Local`](ServiceAccess::Local) reads the host's stand-in. The ONE place
	/// the two-way choice is made, so a declaration runs both ways without any
	/// consumer knowing there are two.
	pub fn runtime_uri(
		&self,
		access: ServiceAccess,
		deploy_id: Option<&Uuid>,
	) -> StoreUri {
		match access {
			ServiceAccess::Remote => self.store_uri(deploy_id),
			ServiceAccess::Local => self.local.clone(),
		}
	}

	/// Component hook deriving both erased halves from the store block on the
	/// same entity: `#[component(immutable, on_insert =
	/// ErasedStoreBlock::on_insert::<Self>, on_remove =
	/// ErasedStoreBlock::on_remove)]`. Runs [`ErasedBlock::on_insert`] first.
	///
	/// Deferred through the command queue because the stack the root resolves
	/// against is an ancestor, and ancestry lands after insertion.
	pub fn on_insert<T: StoreBlock>(mut world: DeferredWorld, cx: HookContext) {
		ErasedBlock::on_insert::<T>(world.reborrow(), cx);
		let entity = cx.entity;
		world.commands().queue(move |world: &mut World| -> Result {
			// tolerate a despawn landing between the insert and this command
			let Ok(entity_ref) = world.get_entity(entity) else {
				return Ok(());
			};
			let block = entity_ref.get_or_else::<T>()?.clone();
			if !world.contains_resource::<PackageConfig>() {
				bevybail!(
					"resolving the store declared as `{}` needs the \
					 `PackageConfig` resource, which `BootstrapPlugin` inserts",
					block.label()
				);
			}
			let stack = world
				.with_state::<StackQuery, _>(|stacks| stacks.resolve(entity));
			world.entity_mut(entity).insert(Self::new(&block, &stack));
			Ok(())
		});
	}

	/// Component hook removing both erased halves with their block.
	pub fn on_remove(mut world: DeferredWorld, cx: HookContext) {
		ErasedBlock::on_remove(world.reborrow(), cx);
		world
			.commands()
			.entity(cx.entity)
			.try_remove::<ErasedStoreBlock>();
	}
}

/// Observer: attach the runtime meaning of a declared store, the concrete
/// provider the declaration's [`ErasedStoreBlock::runtime_uri`] names under
/// this launch's [`ServiceAccess`], built through [`StoreProvider::from_uri`]
/// on every target: a store kind this build has no backend for errors with
/// guidance here rather than silently carrying no store. The provider's own
/// hook lands the erased [`BlobStore`] (and table) beside it.
///
/// Registered by [`InfraPlugin`] rather than hooked on the component, and on
/// the erased half rather than a block type, so a store block defined anywhere
/// (a bucket, a table, a bare uri) attaches without being named here.
/// Deferred through the command queue because the erased half itself lands
/// through it.
pub(crate) fn attach_store(
	ev: On<Insert, ErasedStoreBlock>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let store = entity.get_or_else::<ErasedStoreBlock>()?.clone();
			let deploy_id = entity
				.with_state::<StackQuery, _>(|_, stacks| stacks.deploy_id());
			let uri = store.runtime_uri(
				BootstrapConfig::get().service_access,
				Some(&deploy_id),
			);
			StoreProvider::from_uri(&uri)?.insert(&mut entity);
			Ok(())
		});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A world a store block resolves in: the process identity its stack
	/// composes from, and nothing else.
	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<PackageConfig>();
		world
	}

	fn spawn_store(world: &mut World, block: impl Bundle) -> Entity {
		let stack = world
			.spawn((Stack::new("app").with_stage("prod"), children![block]))
			.id();
		world.flush();
		world.entity(stack).get::<Children>().unwrap()[0]
	}

	/// The runtime half attaches on every target: under the default
	/// [`ServiceAccess::Local`] the declaration lands the host's local
	/// stand-in as a concrete provider, whatever kind the deploy names, keyed
	/// by the composed resource name, and the provider's hook derives the
	/// erased store beside it.
	#[beet_core::test]
	fn attaches_a_local_store() {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		let entity = spawn_store(
			&mut world,
			StoreUriBlock::new("docs", StoreUri::parse("memory://m").unwrap()),
		);
		world.flush();
		world.get::<FsStore>(entity).unwrap().path().xpect_eq(
			ServiceAccess::local_store_dir("app--prod--docs").into_abs(),
		);
		world.get::<BlobStore>(entity).xpect_some();
	}

	/// The two-way choice lives on the erased half: remote is the declared
	/// root (versioned when the store is), local is the stand-in.
	#[beet_core::test]
	fn runtime_uri_picks_by_service_access() {
		let mut world = world();
		let entity = spawn_store(
			&mut world,
			StoreUriBlock::new("repo", StoreUri::parse("s3://bucket").unwrap())
				.with_deploy_versioned(true),
		);
		let erased = world.get::<ErasedStoreBlock>(entity).unwrap();
		let deploy_id = uuid_ext::now_v7();
		erased
			.runtime_uri(ServiceAccess::Remote, Some(&deploy_id))
			.to_string()
			.xpect_eq(format!("s3://bucket/{deploy_id}"));
		erased
			.runtime_uri(ServiceAccess::Local, Some(&deploy_id))
			.xpect_eq(ServiceAccess::local_store_uri("app--prod--repo"));
	}

	/// A declaration naming a memory store names ONE store: every remote read
	/// of it lands on the same backing, so a fixture seeded by name is what
	/// the document reads.
	#[beet_core::test]
	async fn a_memory_declaration_is_one_store() {
		let mut world = world();
		let entity = spawn_store(
			&mut world,
			StoreUriBlock::new(
				"fixtures",
				StoreUri::parse("memory://declared-fixtures").unwrap(),
			),
		);
		let erased = world.get::<ErasedStoreBlock>(entity).unwrap();
		let read = || {
			StoreProvider::from_uri(
				&erased.runtime_uri(ServiceAccess::Remote, None),
			)
			.unwrap()
			.into_blob_store()
		};
		let seeded = read();
		seeded.insert(&RelPath::new("a.txt"), "hi").await.unwrap();
		read()
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from_static(b"hi"));
	}

	/// The erased half is the store's ROOT; the per-deploy prefix is applied by
	/// the consumer holding the id, since the id is the launch's and may change
	/// within it.
	#[beet_core::test]
	fn projects_the_root_and_nests_a_version() {
		let mut world = world();
		let entity = spawn_store(
			&mut world,
			StoreUriBlock::new("repo", StoreUri::parse("s3://bucket").unwrap())
				.with_deploy_versioned(true),
		);
		let erased = world.get::<ErasedStoreBlock>(entity).unwrap();
		erased.root().to_string().xpect_eq("s3://bucket");
		let deploy_id = uuid_ext::now_v7();
		erased
			.store_uri(Some(&deploy_id))
			.to_string()
			.xpect_eq(format!("s3://bucket/{deploy_id}"));
		erased.store_uri(None).to_string().xpect_eq("s3://bucket");
	}

	/// An unversioned store publishes at its root whatever id the consumer
	/// holds, and both halves leave with the block.
	#[beet_core::test]
	fn unversioned_stays_at_the_root_and_removal_takes_both_halves() {
		let mut world = world();
		let entity = spawn_store(
			&mut world,
			StoreUriBlock::new(
				"data",
				StoreUri::parse("fs:/srv/data").unwrap(),
			),
		);
		world
			.get::<ErasedStoreBlock>(entity)
			.unwrap()
			.store_uri(Some(&uuid_ext::now_v7()))
			.to_string()
			.xpect_eq("fs:/srv/data");
		world.entity_mut(entity).remove::<StoreUriBlock>();
		world.flush();
		world.get::<ErasedStoreBlock>(entity).xpect_none();
		world.get::<ErasedBlock>(entity).xpect_none();
	}
}
