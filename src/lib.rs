#![doc = include_str!("../README.md")]
pub use beet_core as core;
pub use beet_flow as ecs;
#[cfg(feature = "ml")]
pub use beet_ml as ml;

pub mod prelude {
	pub use beet_core::prelude::*;
	pub use beet_flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::prelude::*;
}
