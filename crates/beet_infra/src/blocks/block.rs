use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use std::sync::Arc;

/// Trait for infrastructure blocks that generate terraform config.
pub trait Block: 'static + Send + Sync {
	/// Emit this block's resources. `stack` is the resolved identity every name
	/// composes from and `deployment` this launch's mechanics (its id, its
	/// artifacts bucket); `access` carries every [`AccessGrant`] the stack's
	/// blocks declared, so a compute block lowers them to its provider's
	/// permission mechanism and a resource block ignores it.
	fn apply_to_config(
		&self,
		entity: &EntityRef,
		stack: &ResolvedStack,
		deployment: &Deployment,
		access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result;

	/// What a process running in this stack needs to do with this resource,
	/// stated without naming any provider's permission model. Empty for a block
	/// nothing reads at runtime.
	fn runtime_access(&self, _stack: &ResolvedStack) -> Vec<AccessGrant> {
		Vec::new()
	}

	/// If this block creates a deployable artifact, return its label.
	fn artifact_label(&self) -> Option<&str> { None }

	/// Tofu variables declared by this block, resolved
	/// at deploy time and passed via `tofu apply -var`.
	fn variables(&self) -> &[Variable] { &[] }
}

/// Type-erased block for collecting heterogeneous blocks.
/// Stores an [`Arc`] of the original block, allowing access
/// to all [`Block`] trait methods including [`Block::artifact_label`].
#[derive(Clone, Component)]
#[component(immutable)]
pub struct ErasedBlock(Arc<dyn Block>);

impl std::ops::Deref for ErasedBlock {
	type Target = dyn Block;
	fn deref(&self) -> &Self::Target { &*self.0 }
}

impl ErasedBlock {
	pub fn new(block: impl Block) -> Self { Self(Arc::new(block)) }

	/// Component hook that reads a concrete block component from
	/// the entity and inserts an [`ErasedBlock`] wrapping it.
	/// Use with `#[component(on_add = ErasedBlock::on_add::<MyBlock>)]`.
	pub fn on_add<T: Component + Clone + Block>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(block) => {
				world
					.commands()
					.entity(cx.entity)
					.insert(ErasedBlock::new(block));
			}
			Err(err) => {
				world.fallback_error_handler()(err, ErrorContext::Command {
					name: std::any::type_name_of_val(&ErasedBlock::on_add::<T>)
						.into(),
				});
			}
		}
	}
}
