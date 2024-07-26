#![doc = include_str!("../README.md")]
pub use beet_core as core;
pub use beet_ecs as ecs;
#[cfg(feature = "ml")]
pub use beet_ml as ml;

pub mod prelude {
	pub use beet_core::prelude::*;
	pub use beet_ecs::prelude::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::prelude::*;
}
