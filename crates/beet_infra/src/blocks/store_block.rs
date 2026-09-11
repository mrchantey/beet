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
/// uri resolved against the declaration's stack and the versioning flag,
/// inserted by [`on_insert`](Self::on_insert) and removed with the block.
///
/// The ROOT rather than a per-deploy uri, because the deploy id is a property
/// of the launch and may change within it (`<AdoptCurrentDeploy/>` points a
/// content sync at the live version): a consumer applies the id it holds
/// through [`store_uri`](Self::store_uri).
#[derive(Debug, Clone, PartialEq, Get, Component)]
#[component(immutable)]
pub struct ErasedStoreBlock {
	/// The store's root, see [`StoreBlock::store_uri`].
	root: StoreUri,
	/// See [`StoreBlock::deploy_versioned`].
	deploy_versioned: bool,
}

impl ErasedStoreBlock {
	/// The projection of `block` declared under `stack`.
	pub fn new(block: &impl StoreBlock, stack: &ResolvedStack) -> Self {
		Self {
			root: block.store_uri(stack),
			deploy_versioned: block.deploy_versioned(),
		}
	}

	/// The uri a deploy hands a process: the root, nested under `deploy_id`
	/// when the store is versioned. The ONE place that uri is shaped, so the
	/// argv a lambda bakes, the env a release pointer publishes and the prefix a
	/// sync writes to cannot describe the same store differently.
	pub fn store_uri(&self, deploy_id: Option<&Uuid>) -> Result<StoreUri> {
		match (self.deploy_versioned, deploy_id) {
			(true, Some(deploy_id)) => {
				self.root.with_subdir(deploy_id.to_string())
			}
			_ => self.root.clone().xok(),
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

/// Observer: attach the runtime meaning of a declared store. A remote process
/// gets the store the erased uri names ([`BlobStore::from_uri`]), rooted at
/// this launch's deploy version when the store is versioned, which is exactly
/// the uri a deploy bakes for it; a local process gets an [`FsStore`] under
/// `target/stores/<label>`, so one declaration runs both ways whatever its
/// kind.
///
/// Registered by [`InfraPlugin`] rather than hooked on the component, so a
/// backend-free build still carries the declaration. On the erased half rather
/// than a block type, so a store block defined anywhere attaches without being
/// named here. Deferred through the command queue because the erased half
/// itself lands through it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn attach_store(
	ev: On<Insert, ErasedStoreBlock>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let store = entity.get_or_else::<ErasedStoreBlock>()?.clone();
			let label = entity.get_or_else::<ErasedBlock>()?.label.clone();
			match BootstrapConfig::get().service_access {
				ServiceAccess::Remote => {
					let deployment =
						entity.with_state::<StackQuery, _>(|_, stacks| {
							stacks.deployment()
						});
					let uri = store.store_uri(Some(deployment.deploy_id()))?;
					entity.insert(BlobStore::from_uri(
						&uri,
						AbsPathBuf::new(".")?,
					)?);
				}
				ServiceAccess::Local => {
					entity.insert(FsStore::new(
						ServiceAccess::local_store_dir(label.as_str())
							.into_abs(),
					));
				}
			}
			Ok(())
		});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

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
			.unwrap()
			.to_string()
			.xpect_eq(format!("s3://bucket/{deploy_id}"));
		erased
			.store_uri(None)
			.unwrap()
			.to_string()
			.xpect_eq("s3://bucket");
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
			.unwrap()
			.to_string()
			.xpect_eq("fs:/srv/data");
		world.entity_mut(entity).remove::<StoreUriBlock>();
		world.flush();
		world.get::<ErasedStoreBlock>(entity).xpect_none();
		world.get::<ErasedBlock>(entity).xpect_none();
	}
}
