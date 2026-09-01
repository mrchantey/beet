//! The data vocabulary of a deploy block, and the erased half every consumer
//! of "a block, whichever kind" reads.

use crate::prelude::*;
use beet_core::prelude::*;

/// Data vocabulary of a deploy block: no world access, no context bag, never
/// `dyn`. What a block *is* (its label, what it grants, what it declares) lives
/// here; what a block *does* at render time lives in its [`DeployRender`]
/// system, generic for a simple block ([`EmitBlock`]) and bespoke for one with
/// cross-entity inputs (a relation, an artifact, the grant pool).
pub trait Block: Component + Clone {
	/// The label its resource names compose from, ie the `analytics` behind
	/// `beet-site--prod--analytics`.
	fn label(&self) -> &SmolStr;

	/// What a running process needs on this resource, stated without naming any
	/// provider's permission model. Declared by the resource, lowered by the
	/// computes. Empty for a block nothing reads at runtime.
	fn grants(&self, _stack: &ResolvedStack) -> Vec<AccessGrant> { Vec::new() }

	/// Tofu variables declared by this block, resolved at deploy time and
	/// passed via `tofu apply -var`.
	///
	/// Owned rather than borrowed, so a variable a block DERIVES from its label
	/// (a database's master password) need not be stored as a field. A stored
	/// derivation is a field a markup declaration cannot correct: reflect
	/// patches the label over the default and the derived field keeps the
	/// default's.
	fn variables(&self) -> Vec<Variable> { Vec::new() }

	/// If this block creates a deployable artifact, its label: the key the
	/// artifact uploads under and the ledger records.
	fn artifact_label(&self) -> Option<&SmolStr> { None }
}

/// A simple block's emission: pure data in, resources out. Blocks with
/// cross-entity inputs (relations, artifacts, the grant pool) skip this trait
/// and write their own render system instead, which is what stops this
/// signature growing an argument every time one block needs something the
/// others do not.
pub trait EmitBlock: Block {
	/// Emit this block's resources into the config.
	fn emit(
		&self,
		stack: &ResolvedStack,
		deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result;
}

/// The erased half of any [`Block`], inserted by the generic
/// [`on_insert`](Self::on_insert) hook and removed with its block by
/// [`on_remove`](Self::on_remove).
///
/// No `dyn` and no behavior: just the common data a consumer of "a block,
/// whichever kind" needs, ie [`TofuApplyAction`] pairing a [`BuildArtifact`]
/// with the label its block declared. Blocks are immutable components, so
/// reinsertion is the only way a block changes and the `on_insert` edge is
/// exactly every path by which this projection could go stale.
#[derive(Debug, Clone, Component)]
#[component(immutable)]
pub struct ErasedBlock {
	/// The concrete block's short type name, for diagnostics.
	pub type_name: &'static str,
	/// The block's [`label`](Block::label).
	pub label: SmolStr,
	/// The block's [`artifact_label`](Block::artifact_label).
	pub artifact_label: Option<SmolStr>,
}

impl ErasedBlock {
	/// Component hook deriving the erased half from the block on the same
	/// entity: `#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	/// on_remove = ErasedBlock::on_remove)]`.
	///
	/// A declaration entity holds at most one block (both its meanings hang off
	/// the one entity), so a second block TYPE raises a clobber error rather
	/// than silently retagging the erased half; reinserting the same type is a
	/// refresh.
	pub fn on_insert<T: Block>(mut world: DeferredWorld, cx: HookContext) {
		let entity = cx.entity;
		world.commands().queue(move |world: &mut World| -> Result {
			// tolerate a despawn landing between the insert and this command
			let Ok(entity_ref) = world.get_entity(entity) else {
				return Ok(());
			};
			let existing = entity_ref
				.get::<ErasedBlock>()
				.map(|erased| erased.type_name);
			let block = entity_ref.get_or_else::<T>()?.clone();
			if let Some(existing) = existing
				&& existing != short_type_name::<T>()
			{
				bevybail!(
					"an entity holds at most one block, but {entity} already \
					 declares a {existing}. Remove it before inserting a {}.",
					short_type_name::<T>()
				);
			}
			let erased = Self {
				type_name: short_type_name::<T>(),
				label: block.label().clone(),
				artifact_label: block.artifact_label().cloned(),
			};
			world.entity_mut(entity).insert(erased);
			Ok(())
		});
	}

	/// Component hook removing the erased half with its block. A replace
	/// (reinsert) fires `on_discard` then `on_insert`, never this, so a
	/// refresh keeps the projection; only a true removal (or despawn, which
	/// the `try_` tolerates) clears it.
	pub fn on_remove(mut world: DeferredWorld, cx: HookContext) {
		let entity = cx.entity;
		world.commands().entity(entity).try_remove::<ErasedBlock>();
	}
}

/// The short type name of `T`, ie `S3BucketBlock`.
pub(crate) fn short_type_name<T>() -> &'static str {
	core::any::type_name::<T>()
		.rsplit("::")
		.next()
		.unwrap_or_default()
}

#[cfg(all(
	test,
	feature = "bindings_aws_common",
	feature = "bindings_aws_dynamo"
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Blocks are immutable components, so reinsertion is the only mutation
	/// path, and the `on_insert` edge refreshes the projection on exactly that
	/// path: the erased half cannot go stale.
	#[beet_core::test]
	fn reinsertion_refreshes_the_erased_half() {
		let mut world = World::new();
		let entity = world.spawn(S3BucketBlock::new("app")).id();
		world.flush();
		world
			.get::<ErasedBlock>(entity)
			.unwrap()
			.label
			.as_str()
			.xpect_eq("app");
		world.entity_mut(entity).insert(S3BucketBlock::new("ops"));
		world.flush();
		world
			.get::<ErasedBlock>(entity)
			.unwrap()
			.label
			.as_str()
			.xpect_eq("ops");
	}

	/// The projection leaves with its block; a lingering `ErasedBlock` would
	/// keep pairing artifacts for a declaration that no longer exists.
	#[beet_core::test]
	fn removal_takes_the_erased_half() {
		let mut world = World::new();
		let entity = world.spawn(S3BucketBlock::new("app")).id();
		world.flush();
		world.get::<ErasedBlock>(entity).xpect_some();
		world.entity_mut(entity).remove::<S3BucketBlock>();
		world.flush();
		world.get::<ErasedBlock>(entity).xpect_none();
	}

	/// A declaration entity holds at most one block: a second TYPE raises
	/// rather than silently retagging the erased half.
	#[beet_core::test]
	#[should_panic = "holds at most one block"]
	fn a_second_block_type_raises() {
		let mut world = World::new();
		world.spawn((
			S3BucketBlock::new("app"),
			DynamoTableBlock::new("analytics"),
		));
		world.flush();
	}
}
