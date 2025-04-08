#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![doc = include_str!("../README.md")]
#[cfg(feature = "design")]
pub use beet_design as design;
#[cfg(feature = "examples")]
pub use beet_examples as examples;
#[cfg(feature = "flow")]
pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "rsx")]
pub use beet_rsx as rsx;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use beet_server as server;
#[cfg(feature = "sim")]
pub use beet_sim as sim;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;

pub mod prelude {
	#[cfg(feature = "design")]
	pub use crate::design::prelude::*;
	#[cfg(feature = "examples")]
	pub use crate::examples::prelude::*;
	#[cfg(feature = "flow")]
	pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	#[cfg(feature = "rsx")]
	pub use crate::rsx::prelude::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::prelude::*;
	#[cfg(feature = "sim")]
	pub use crate::sim::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
}


pub mod exports {
	#[cfg(feature = "rsx")]
	pub use beet_rsx::exports::*;
	// #[cfg(feature = "examples")]
	// pub use beet_examples::exports::*;
	// #[cfg(feature = "flow")]
	// pub use beet_flow::exports::*;
	// #[cfg(feature = "ml")]
	// pub use beet_ml::exports::*;
	// #[cfg(feature = "router")]
	// pub use beet_router::exports::*;
	// #[cfg(feature = "server")]
	// pub use beet_server::exports::*;
	// #[cfg(feature = "sim")]
	// pub use beet_sim::exports::*;
	// #[cfg(feature = "spatial")]
	// pub use beet_spatial::exports::*;
}
