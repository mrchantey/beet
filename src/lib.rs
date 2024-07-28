#![doc = include_str!("../README.md")]
pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
pub use beet_spatial as spatial;

pub mod prelude {
	pub use beet_flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::prelude::*;
	pub use beet_spatial::prelude::*;
}
