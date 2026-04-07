use crate::prelude::*;
use beet_core::prelude::*;

pub trait Block {
	fn apply_to_config(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result;
}
