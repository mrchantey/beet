#![doc = include_str!("../README.md")]
#[cfg(feature = "flow")]
pub use beet_flow as flow;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "rsx")]
pub use beet_rsx as rsx;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;
#[cfg(feature = "sim")]
pub use beet_sim as sim;

pub mod prelude {
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	#[cfg(feature = "flow")]
	pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	#[cfg(feature = "rsx")]
	pub use crate::rsx::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
	#[cfg(feature = "sim")]
	pub use crate::sim::prelude::*;
	#[cfg(feature = "sim")]
	pub use crate::sim::prelude::*;
}
