use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

pub trait Block {
	fn apply_to_config(
		&self,
		entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result;
}

/// Added by blocks for collecting.
#[derive(Clone, Component)]
#[component(immutable)]
pub struct ErasedBlock(
	Arc<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(&EntityRef, &Stack, &mut terra::Config) -> Result,
	>,
);


impl ErasedBlock {
	pub fn new<T: Component + Block>() -> Self {
		Self(Arc::new(|entity, stack, config| {
			let block = entity.get::<T>().ok_or_else(|| {
				bevyhow!(
					"Block component missing: {}",
					std::any::type_name::<T>()
				)
			})?;
			block.apply_to_config(entity, stack, config)?;
			Ok(())
		}))
	}

	pub fn on_add<T: 'static + Send + Sync + Clone + Component + Block>()
	-> impl FnOnce(DeferredWorld, HookContext) {
		|mut world, cx| {
			let block = world
				.entity(cx.entity)
				.get::<T>()
				.expect(&format!(
					"Block component missing: {}",
					std::any::type_name::<T>()
				))
				.clone();
			// TODO something like this
			// world
			// 	.commands()
			// 	.entity(cx.entity)
			// 	.insert(ErasedBlock::new(block));
		}
	}

	pub fn apply_to_config(
		&self,
		entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		(self.0)(entity, stack, config)
	}
}
