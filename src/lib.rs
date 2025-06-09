#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![doc = include_str!("../README.md")]
#[cfg(feature = "build")]
pub use beet_build as build;
pub use beet_common as common;
#[cfg(feature = "connect")]
pub use beet_connect as connect;
#[cfg(feature = "design")]
pub use beet_design as design;
#[cfg(feature = "examples")]
pub use beet_examples as examples;
#[cfg(feature = "flow")]
pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "parse")]
pub use beet_parse as parse;
#[cfg(feature = "query")]
pub use beet_query as query;
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

// #[cfg(all(feature = "router", not(target_arch = "wasm32")))]
// mod default_runner;

pub mod prelude {
	#[cfg(feature = "build")]
	pub use crate::build::prelude::*;
	pub use crate::common::prelude::*;
	#[cfg(feature = "connect")]
	pub use crate::connect::prelude::*;
	#[cfg(feature = "design")]
	pub use crate::design::prelude::*;
	#[cfg(feature = "examples")]
	pub use crate::examples::prelude::*;
	#[cfg(feature = "flow")]
	pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	#[cfg(feature = "parse")]
	pub use crate::parse::prelude::*;
	#[cfg(feature = "query")]
	pub use crate::query::prelude::*;
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


	// #[cfg(all(feature = "router", not(target_arch = "wasm32")))]
	// pub use crate::default_runner::*;
}


pub mod exports {
	#[cfg(feature = "build")]
	pub use beet_build::exports::*;
	#[cfg(feature = "router")]
	pub use beet_router::exports::*;
	#[cfg(feature = "rsx")]
	pub use beet_rsx::exports::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use beet_server::exports::*;
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


#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
