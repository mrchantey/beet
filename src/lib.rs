#![doc = include_str!("../README.md")]
pub use beet_core;
pub use beet_ecs;
#[cfg(feature = "ml")]
pub use beet_ml;
#[cfg(feature = "net")]
pub use beet_net;

pub mod prelude {

	pub use beet_core::prelude::*;
	pub use beet_ecs::prelude::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::prelude::*;
	#[cfg(feature = "net")]
	pub use beet_net::prelude::*;
}
