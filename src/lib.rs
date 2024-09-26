#![doc = include_str!("../README.md")]
pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
pub use beet_spatial as spatial;
pub mod plugins;

pub mod prelude {
	pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	pub use crate::spatial::prelude::*;
	pub use crate::plugins::*;
}
