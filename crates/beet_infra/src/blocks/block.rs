use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

pub trait Block {
	fn apply_to_config(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result;
}

/// Added by blocks for collecting.
#[derive(Clone, Component)]
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
			block.apply_to_config(stack, config)?;
			Ok(())
		}))
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
