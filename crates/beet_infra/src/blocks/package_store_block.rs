//! The deploy-side declaration of the package store, and how a consumer finds it.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marks the store block an app is served from: the one [`StoreBlock`] the
/// deploy publishes the entry document to and every compute it ships boots
/// from, ie `<S3BucketBlock label="package" {PackageStoreBlock}/>`.
///
/// A marker on the store's own declaration rather than a block of its own,
/// because the store may be any [`StoreBlock`]: a bucket the deploy creates, a
/// [`StoreUriBlock`] naming one it does not. The only thing left to say is
/// WHICH, and consumers find it by this type ([`PackageStoreQuery`]) rather than
/// by a label convention. One per world, enforced by its own insert hook
/// exactly as the runtime `PackageStore` is.
///
/// Distinct from `PackageStore`, which marks the live `BlobStore` a process
/// actually booted from. That store exists before any document is read (the
/// document is read out of it), so a declaration inside the document can
/// describe the package store but never be it. Not a [`Block`] despite the name:
/// it reads as "the block that is the package store", which is what it marks.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::exclusive::<PackageStoreBlock>())]
pub struct PackageStoreBlock;

impl PackageStoreBlock {
	/// The error for a marker on an entity declaring no store block, shared by
	/// the render assertion and the query so the two cannot drift.
	fn storeless(entity: Entity) -> BevyError {
		bevyhow!(
			"entity {entity} is marked `PackageStoreBlock` but declares no store \
			 block: the marker goes on the store's own declaration, ie \
			 `<S3BucketBlock label=\"package\" {{PackageStoreBlock}}/>`"
		)
	}
}

/// Render-set: a marker on an entity declaring no store block is a collected
/// error, so `validate` names it before any deploy consults the store.
pub(crate) fn assert_package_store_blocks(
	mut scopes: AncestorQuery<&mut RenderScope>,
	markers: Query<
		Entity,
		(With<PackageStoreBlock>, Without<ErasedStoreBlock>),
	>,
) {
	for entity in markers.iter() {
		if let Ok(mut scope) = scopes.get_mut(entity) {
			scope.error(PackageStoreBlock::storeless(entity));
		}
	}
}

/// The declaration a consumer resolved: the entity carrying
/// [`PackageStoreBlock`], its label and its erased store half. No launch state
/// lives here; the uri this launch publishes at is read through
/// [`PackageStoreQuery::store_uri`] as it is needed.
#[derive(Debug, Clone, Get)]
pub struct PackageStoreDecl {
	entity: Entity,
	/// The declaration's [`label`](Block::label).
	label: SmolStr,
	store: ErasedStoreBlock,
}

impl PackageStoreDecl {
	/// The store's root, every version of the document below it.
	pub fn root(&self) -> &StoreUri { self.store.root() }

	/// Whether each deploy publishes under its own id, see
	/// [`StoreBlock::deploy_versioned`].
	pub fn deploy_versioned(&self) -> bool { self.store.deploy_versioned() }

	/// The bucket the store is, for a consumer that can only address S3: the
	/// box's `aws s3 cp`, a function's `s3_bucket`. Any other kind is an error
	/// naming it, since such a consumer cannot boot from it.
	pub fn bucket_name(&self) -> Result<SmolStr> {
		match self.root() {
			StoreUri::S3 { name, .. } => name.clone().xok(),
			other => bevybail!(
				"the package store `{}` is `{other}`, but this compute reads its \
				 binary from an s3:// bucket: declare it as one, ie \
				 `<S3BucketBlock label=\"package\" {{PackageStoreBlock}}/>`",
				self.label
			),
		}
	}
}

#[cfg(test)]
impl PackageStoreBlock {
	/// The bucket [`test_store`](Self::test_store) names.
	pub(crate) const TEST_BUCKET: &'static str = "test-package";

	/// A versioned S3 package store for a render test, declared as a bare uri so
	/// it emits no resource and no grant of its own.
	pub(crate) fn test_store() -> impl Bundle {
		(
			StoreUriBlock::new(
				"package",
				StoreUri::parse(&format!("s3://{}", Self::TEST_BUCKET))
					.unwrap(),
			)
			.with_deploy_versioned(true),
			PackageStoreBlock,
		)
	}
}

/// Resolves the [`PackageStoreBlock`] a consumer reads.
///
/// A template needing the store at build (a compute baking the uri it boots
/// from) resolves it in an `On<Ready>` observer rather than at spawn: a sibling
/// declaration, possibly a forward one, lands its erased half through the
/// command queue, and by [`Ready`] every declaration in the document has.
#[derive(SystemParam)]
pub struct PackageStoreQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	package_stores: Query<
		'w,
		's,
		(
			Entity,
			Option<&'static ErasedBlock>,
			Option<&'static ErasedStoreBlock>,
		),
		With<PackageStoreBlock>,
	>,
}

impl PackageStoreQuery<'_, '_> {
	/// The package store declared under `entity`'s stack, `None` when it declares
	/// none: a stack that serves no document (a bucket-only example) has no
	/// package store, and one under ANOTHER stack is that stack's. A consumer
	/// outside every stack sees the process's one.
	pub fn find(&self, entity: Entity) -> Result<Option<PackageStoreDecl>> {
		let declared = self.stacks.declared(entity).ok();
		let Some((marker, erased, store)) =
			self.package_stores.iter().find(|(marker, ..)| {
				declared
					.as_ref()
					.is_none_or(|declared| declared.contains(marker))
			})
		else {
			return Ok(None);
		};
		let (Some(erased), Some(store)) = (erased, store) else {
			return Err(PackageStoreBlock::storeless(marker));
		};
		Some(PackageStoreDecl {
			entity: marker,
			label: erased.label.clone(),
			// the stack as this query sees it rather than as the declaration
			// landed, so a render at another stage names that stage's store
			store: store.at(&self.stacks.resolve(marker))?,
		})
		.xok()
	}

	/// [`find`](Self::find) for a consumer that cannot do without it, ie a
	/// compute baking the uri it boots from: an undeclared package store is an
	/// error naming what to declare and where.
	pub fn get(&self, entity: Entity) -> Result<PackageStoreDecl> {
		self.find(entity)?.ok_or_else(|| {
			match self.package_stores.iter().next() {
				Some((other, ..)) => bevyhow!(
					"the package store is declared on entity {other}, under a \
					 different stack than this consumer's: a compute only reads \
					 the stores declared under its own `<Stack>`, so declare it \
					 there"
				),
				None => bevyhow!(
					"no package store declared: mark the store block the app is \
					 served from, ie `<S3BucketBlock label=\"package\" \
					 {{PackageStoreBlock}}/>`"
				),
			}
		})
	}

	/// The uri a process this launch deploys boots from: the root nested under
	/// this launch's version when the store is versioned, see
	/// [`ErasedStoreBlock::store_uri`]. The id is read as this is called, so a
	/// launch that adopts or rolls back to another version
	/// ([`Deployment::update_from_ledger`]) bakes that version's uri.
	pub fn store_uri(&self, entity: Entity) -> Result<StoreUri> {
		self.get(entity)?
			.store
			.store_uri(Some(&self.stacks.deploy_id()))
			.xok()
	}

	/// The client publishing this launch's version into the package store, `None`
	/// for a stack declaring no package store or an unversioned one: a store read
	/// at its root has no version prefix to hold a ledger. Its ledger records
	/// the document at [`store_uri`](Self::store_uri), so the release pointer
	/// publishes the same uri a compute bakes.
	pub fn find_artifacts_client(
		&self,
		entity: Entity,
	) -> Result<Option<ArtifactsClient>> {
		let Some(package_store) = self
			.find(entity)?
			.filter(|package_store| package_store.deploy_versioned())
		else {
			return Ok(None);
		};
		let deployment = self.stacks.deployment();
		let ledger = ArtifactLedger::new(
			*deployment.deploy_id(),
			deployment.deploy_timestamp().to_string(),
		)
		.with_package(
			package_store.store.store_uri(Some(deployment.deploy_id())),
		);
		ArtifactsClient::new(BlobStore::from_uri(package_store.root())?, ledger)
			.xmap(Some)
			.xok()
	}

	/// [`find_artifacts_client`](Self::find_artifacts_client) for a verb that
	/// cannot do without one (an artifact upload, an adopt, a rollback): an
	/// undeclared or unversioned package store is an error naming what to declare.
	pub fn artifacts_client(&self, entity: Entity) -> Result<ArtifactsClient> {
		self.find_artifacts_client(entity)?.ok_or_else(|| {
			match self.get(entity) {
				Ok(package_store) => bevyhow!(
					"the package store `{}` is not deploy-versioned, so it has no \
					 version prefix to hold a ledger or a binary: declare it \
					 `deploy_versioned=true`",
					package_store.label()
				),
				Err(err) => err,
			}
		})
	}

	/// The [`BootstrapConfig`] a process this launch deploys boots with: the
	/// package store at [`store_uri`](Self::store_uri) and nothing else, a deploy
	/// target adding only what its transport needs.
	///
	/// ## Why the deploy id is baked in rather than resolved at run time
	///
	/// A deploy publishes the site into the store and THEN swaps the binary that
	/// serves it, because the store has to hold the site before the process
	/// that reads it exists. At one mutable location that ordering is a window
	/// in which the OLD binary reads the NEW document, which is a hard parse
	/// failure the moment the document uses syntax that binary predates. Giving
	/// each deploy its own prefix closes the window: a binary only ever reads
	/// the document it shipped with.
	///
	/// It also makes a rollback whole for free. Re-applying with an earlier
	/// deploy id swaps the process back to that version's artifact, and that
	/// artifact was baked pointing at that version's document, so the binary
	/// and the document move together with nothing keeping them in step.
	///
	/// The prefix, the sync's destination and the ledger's record all derive
	/// from the store's one erased declaration, so they agree by construction;
	/// a store declaring `deploy_versioned=false` is read at its root by every
	/// one of them.
	///
	/// A deploy target that cannot bake a per-deploy value into its own boot
	/// config does not call this at all. It sets no `package`, and the release
	/// pointer it resolves per start publishes one instead: see
	/// [`ArtifactLedger::package`].
	pub fn bootstrap(&self, entity: Entity) -> Result<BootstrapConfig> {
		BootstrapConfig {
			package: Some(self.store_uri(entity)?),
			..default()
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn world() -> World {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		world
	}

	fn package_block() -> StoreUriBlock {
		StoreUriBlock::new("package", StoreUri::parse("s3://bucket").unwrap())
			.with_deploy_versioned(true)
	}

	/// A stack declaring `blocks`, returning the entity of a consumer declared
	/// beside them.
	fn stack_with(world: &mut World, blocks: impl Bundle) -> Entity {
		let stack = world
			.spawn((Stack::new("app"), children![
				blocks,
				Name::new("consumer")
			]))
			.id();
		world.flush();
		world
			.entity(stack)
			.get::<Children>()
			.unwrap()
			.iter()
			.find(|child| world.get::<Name>(*child).is_some())
			.unwrap()
	}

	/// The uri a consumer under `world`'s one stack boots from.
	fn store_uri(world: &mut World, consumer: Entity) -> String {
		world
			.with_state::<PackageStoreQuery, _>(|package_stores| {
				package_stores.store_uri(consumer)
			})
			.unwrap()
			.to_string()
	}

	/// A consumer under the stack finds the marked store by type and reads the
	/// uri this launch publishes it at.
	#[beet_core::test]
	fn resolves_the_marked_store() {
		let mut world = world();
		let consumer =
			stack_with(&mut world, (package_block(), PackageStoreBlock));
		let deploy_id = *world.resource::<Deployment>().deploy_id();
		let package_store = world
			.with_state::<PackageStoreQuery, _>(|package_stores| {
				package_stores.get(consumer)
			})
			.unwrap();
		package_store.label().as_str().xpect_eq("package");
		package_store.root().to_string().xpect_eq("s3://bucket");
		package_store
			.bucket_name()
			.unwrap()
			.as_str()
			.xpect_eq("bucket");
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{deploy_id}/package"));
	}

	/// The client publishes into the declared store, and its ledger names the
	/// same document uri a compute bakes. An unversioned store has no version
	/// prefix to publish into, so a verb needing the client is told why.
	#[beet_core::test]
	fn artifacts_client_publishes_into_the_package_store() {
		let mut world = world();
		let memory = StoreUri::parse("memory://package-client").unwrap();
		let consumer = stack_with(
			&mut world,
			(
				StoreUriBlock::new("package", memory.clone())
					.with_deploy_versioned(true),
				PackageStoreBlock,
			),
		);
		let deploy_id = *world.resource::<Deployment>().deploy_id();
		let client = world
			.with_state::<PackageStoreQuery, _>(|package_stores| {
				package_stores.artifacts_client(consumer)
			})
			.unwrap();
		client
			.store()
			.root_key()
			.xpect_eq(BlobStore::from_uri(&memory).unwrap().root_key());
		client
			.ledger()
			.package
			.as_ref()
			.unwrap()
			.to_string()
			.xpect_eq(format!("memory://package-client/{deploy_id}/package"));
	}

	/// An unversioned store has no version prefix to publish into, so a verb
	/// needing the client is told why rather than writing beside the document.
	#[beet_core::test]
	fn an_unversioned_store_has_no_client() {
		let mut world = world();
		let consumer = stack_with(
			&mut world,
			(
				StoreUriBlock::new(
					"package",
					StoreUri::parse("memory://package-unversioned").unwrap(),
				),
				PackageStoreBlock,
			),
		);
		world.with_state::<PackageStoreQuery, _>(|package_stores| {
			package_stores
				.find_artifacts_client(consumer)
				.unwrap()
				.xpect_none();
			package_stores
				.artifacts_client(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("not deploy-versioned");
		});
	}

	/// The uri nests under the id the launch holds WHEN it is read, so a launch
	/// that adopts or rolls back to another version bakes that version.
	#[beet_core::test]
	fn reads_the_deploy_id_when_called() {
		let mut world = world();
		let consumer =
			stack_with(&mut world, (package_block(), PackageStoreBlock));
		let first = *world.resource::<Deployment>().deploy_id();
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{first}/package"));
		let ledger = ArtifactLedger::new(uuid_ext::now_v7(), "later".into());
		world
			.resource_mut::<Deployment>()
			.update_from_ledger(&ledger);
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{}/package", ledger.deploy_id));
	}

	/// A stack declaring no package store finds none, and a consumer that cannot
	/// do without one is told what to declare.
	#[beet_core::test]
	fn an_undeclared_store_is_none_or_a_named_error() {
		let mut world = world();
		let consumer = stack_with(&mut world, package_block());
		world.with_state::<PackageStoreQuery, _>(|package_stores| {
			package_stores.find(consumer).unwrap().xpect_none();
			package_stores
				.get(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("no package store declared");
		});
	}

	/// Another stack's package store is that stack's: a consumer elsewhere finds
	/// none, and the error names where the store actually is.
	#[beet_core::test]
	fn another_stacks_store_is_not_this_ones() {
		let mut world = world();
		stack_with(&mut world, (package_block(), PackageStoreBlock));
		let consumer = stack_with(&mut world, ());
		world.with_state::<PackageStoreQuery, _>(|package_stores| {
			package_stores.find(consumer).unwrap().xpect_none();
			package_stores
				.get(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("under a different stack");
		});
	}

	/// The marker belongs on a store block: alone it is an error at the query
	/// and at the render, so `validate` reports it before any deploy.
	#[beet_core::test]
	fn a_storeless_marker_errors() {
		let mut world = world();
		let consumer = stack_with(&mut world, PackageStoreBlock);
		world.with_state::<PackageStoreQuery, _>(|package_stores| {
			package_stores
				.find(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("declares no store block");
		});
		RenderScope::test_render(|parent| {
			parent.spawn(PackageStoreBlock);
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("declares no store block");
	}

	/// One package store per world, whatever the stacks.
	#[beet_core::test]
	#[should_panic = "exactly one `PackageStoreBlock`"]
	fn rejects_a_second_marker() {
		let mut world = world();
		world.spawn((package_block(), PackageStoreBlock));
		world.spawn((package_block(), PackageStoreBlock));
		world.flush();
	}
}
