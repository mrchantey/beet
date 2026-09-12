//! The deploy-side declaration of the repo store, and how a consumer finds it.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marks the store block an app is served from: the one [`StoreBlock`] the
/// deploy publishes the entry document to and every compute it ships boots
/// from, ie `<S3BucketBlock label="repo" {RepoStoreBlock}/>`.
///
/// A marker on the store's own declaration rather than a block of its own,
/// because the store may be any [`StoreBlock`]: a bucket the deploy creates, a
/// [`StoreUriBlock`] naming one it does not. The only thing left to say is
/// WHICH, and consumers find it by this type ([`RepoStoreQuery`]) rather than
/// by a label convention. One per world, enforced by its own insert hook
/// exactly as the runtime `RepoStore` is.
///
/// Distinct from `RepoStore`, which marks the live `BlobStore` a process
/// actually booted from. That store exists before any document is read (the
/// document is read out of it), so a declaration inside the document can
/// describe the repo store but never be it. Not a [`Block`] despite the name:
/// it reads as "the block that is the repo store", which is what it marks.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = hook_ext::exclusive::<RepoStoreBlock>())]
pub struct RepoStoreBlock;

impl RepoStoreBlock {
	/// The error for a marker on an entity declaring no store block, shared by
	/// the render assertion and the query so the two cannot drift.
	fn storeless(entity: Entity) -> BevyError {
		bevyhow!(
			"entity {entity} is marked `RepoStoreBlock` but declares no store \
			 block: the marker goes on the store's own declaration, ie \
			 `<S3BucketBlock label=\"repo\" {{RepoStoreBlock}}/>`"
		)
	}
}

/// Render-set: a marker on an entity declaring no store block is a collected
/// error, so `validate` names it before any deploy consults the store.
pub(crate) fn assert_repo_store_blocks(
	mut scopes: AncestorQuery<&mut RenderScope>,
	markers: Query<Entity, (With<RepoStoreBlock>, Without<ErasedStoreBlock>)>,
) {
	for entity in markers.iter() {
		if let Ok(mut scope) = scopes.get_mut(entity) {
			scope.error(RepoStoreBlock::storeless(entity));
		}
	}
}

/// The declaration a consumer resolved: the entity carrying
/// [`RepoStoreBlock`], its label and its erased store half. No launch state
/// lives here; the uri this launch publishes at is read through
/// [`RepoStoreQuery::store_uri`] as it is needed.
#[derive(Debug, Clone, Get)]
pub struct RepoStoreDecl {
	entity: Entity,
	/// The declaration's [`label`](Block::label).
	label: SmolStr,
	store: ErasedStoreBlock,
}

impl RepoStoreDecl {
	/// The store's root, every version of the document below it.
	pub fn root(&self) -> &StoreUri { self.store.root() }
}

/// Resolves the [`RepoStoreBlock`] a consumer reads.
///
/// A template needing the store at build (a compute baking the uri it boots
/// from) resolves it in an `On<Ready>` observer rather than at spawn: a sibling
/// declaration, possibly a forward one, lands its erased half through the
/// command queue, and by [`Ready`] every declaration in the document has.
#[derive(SystemParam)]
pub struct RepoStoreQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	repos: Query<
		'w,
		's,
		(
			Entity,
			Option<&'static ErasedBlock>,
			Option<&'static ErasedStoreBlock>,
		),
		With<RepoStoreBlock>,
	>,
}

impl RepoStoreQuery<'_, '_> {
	/// The repo store declared under `entity`'s stack, `None` when it declares
	/// none: a stack that serves no document (a bucket-only example) has no
	/// repo store, and one under ANOTHER stack is that stack's. A consumer
	/// outside every stack sees the process's one.
	pub fn find(&self, entity: Entity) -> Result<Option<RepoStoreDecl>> {
		let declared = self.stacks.declared(entity).ok();
		let Some((marker, erased, store)) =
			self.repos.iter().find(|(marker, ..)| {
				declared
					.as_ref()
					.is_none_or(|declared| declared.contains(marker))
			})
		else {
			return Ok(None);
		};
		let (Some(erased), Some(store)) = (erased, store) else {
			return Err(RepoStoreBlock::storeless(marker));
		};
		Some(RepoStoreDecl {
			entity: marker,
			label: erased.label.clone(),
			store: store.clone(),
		})
		.xok()
	}

	/// [`find`](Self::find) for a consumer that cannot do without it, ie a
	/// compute baking the uri it boots from: an undeclared repo store is an
	/// error naming what to declare and where.
	pub fn get(&self, entity: Entity) -> Result<RepoStoreDecl> {
		self.find(entity)?
			.ok_or_else(|| match self.repos.iter().next() {
				Some((other, ..)) => bevyhow!(
					"the repo store is declared on entity {other}, under a \
					 different stack than this consumer's: a compute only reads \
					 the stores declared under its own `<Stack>`, so declare it \
					 there"
				),
				None => bevyhow!(
					"no repo store declared: mark the store block the app is \
					 served from, ie `<S3BucketBlock label=\"repo\" \
					 {{RepoStoreBlock}}/>`"
				),
			})
	}

	/// The uri a process this launch deploys boots from: the root nested under
	/// this launch's deploy id when the store is versioned, see
	/// [`ErasedStoreBlock::store_uri`]. The id is read as this is called, so a
	/// launch that adopts or rolls back to another version
	/// ([`Deployment::update_from_ledger`]) bakes that version's uri.
	pub fn store_uri(&self, entity: Entity) -> Result<StoreUri> {
		self.get(entity)?
			.store
			.store_uri(Some(&self.stacks.deploy_id()))
	}

	/// The [`BootstrapConfig`] a process this launch deploys boots with: the
	/// repo store at [`store_uri`](Self::store_uri) and nothing else, a deploy
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
	/// config does not call this at all. It sets no `repo`, and the release
	/// pointer it resolves per start publishes one instead: see
	/// [`ArtifactLedger::repo`].
	pub fn bootstrap(&self, entity: Entity) -> Result<BootstrapConfig> {
		BootstrapConfig {
			repo: Some(self.store_uri(entity)?),
			..default()
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn world() -> World {
		let mut world = InfraPlugin.into_world();
		world.init_resource::<PackageConfig>();
		world
	}

	fn repo_block() -> StoreUriBlock {
		StoreUriBlock::new("repo", StoreUri::parse("s3://bucket").unwrap())
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
			.with_state::<RepoStoreQuery, _>(|repos| repos.store_uri(consumer))
			.unwrap()
			.to_string()
	}

	/// A consumer under the stack finds the marked store by type and reads the
	/// uri this launch publishes it at.
	#[beet_core::test]
	fn resolves_the_marked_store() {
		let mut world = world();
		let consumer = stack_with(&mut world, (repo_block(), RepoStoreBlock));
		let deploy_id = *world.resource::<Deployment>().deploy_id();
		let repo = world
			.with_state::<RepoStoreQuery, _>(|repos| repos.get(consumer))
			.unwrap();
		repo.label().as_str().xpect_eq("repo");
		repo.root().to_string().xpect_eq("s3://bucket");
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{deploy_id}"));
	}

	/// The uri nests under the id the launch holds WHEN it is read, so a launch
	/// that adopts or rolls back to another version bakes that version.
	#[beet_core::test]
	fn reads_the_deploy_id_when_called() {
		let mut world = world();
		let consumer = stack_with(&mut world, (repo_block(), RepoStoreBlock));
		let first = *world.resource::<Deployment>().deploy_id();
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{first}"));
		let ledger = ArtifactLedger::new(uuid_ext::now_v7(), "later".into());
		world
			.resource_mut::<Deployment>()
			.update_from_ledger(&ledger);
		store_uri(&mut world, consumer)
			.xpect_eq(format!("s3://bucket/{}", ledger.deploy_id));
	}

	/// A stack declaring no repo store finds none, and a consumer that cannot
	/// do without one is told what to declare.
	#[beet_core::test]
	fn an_undeclared_store_is_none_or_a_named_error() {
		let mut world = world();
		let consumer = stack_with(&mut world, repo_block());
		world.with_state::<RepoStoreQuery, _>(|repos| {
			repos.find(consumer).unwrap().xpect_none();
			repos
				.get(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("no repo store declared");
		});
	}

	/// Another stack's repo store is that stack's: a consumer elsewhere finds
	/// none, and the error names where the store actually is.
	#[beet_core::test]
	fn another_stacks_store_is_not_this_ones() {
		let mut world = world();
		stack_with(&mut world, (repo_block(), RepoStoreBlock));
		let consumer = stack_with(&mut world, ());
		world.with_state::<RepoStoreQuery, _>(|repos| {
			repos.find(consumer).unwrap().xpect_none();
			repos
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
		let consumer = stack_with(&mut world, RepoStoreBlock);
		world.with_state::<RepoStoreQuery, _>(|repos| {
			repos
				.find(consumer)
				.unwrap_err()
				.to_string()
				.xpect_contains("declares no store block");
		});
		RenderScope::test_render(|parent| {
			parent.spawn(RepoStoreBlock);
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("declares no store block");
	}

	/// One repo store per world, whatever the stacks.
	#[beet_core::test]
	#[should_panic = "exactly one `RepoStoreBlock`"]
	fn rejects_a_second_marker() {
		let mut world = world();
		world.spawn((repo_block(), RepoStoreBlock));
		world.spawn((repo_block(), RepoStoreBlock));
		world.flush();
	}
}
