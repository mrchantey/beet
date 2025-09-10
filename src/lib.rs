#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![doc = include_str!("../README.md")]
mod beet_plugins;

#[cfg(feature = "agent")]
pub use beet_agent as agent;
#[cfg(feature = "build")]
pub use beet_build as build;
pub use beet_core as core;
#[cfg(feature = "design")]
pub use beet_design as design;
#[cfg(feature = "dom")]
pub use beet_dom as dom;
#[cfg(feature = "examples")]
pub use beet_examples as examples;
#[cfg(feature = "flow")]
pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "parse")]
pub use beet_parse as parse;
// #[cfg(feature = "query")]
// pub use beet_query as query;
#[cfg(feature = "net")]
pub use beet_net as net;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "rsx")]
pub use beet_rsx as rsx;
#[cfg(feature = "sim")]
pub use beet_sim as sim;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;
pub use beet_utils as utils;
pub use beet_utils::elog;
pub use beet_utils::log;
pub use beet_utils::noop;
pub mod prelude {
	#[cfg(feature = "agent")]
	pub use crate::agent::prelude::*;
	pub use crate::beet_plugins::*;
	#[cfg(feature = "build")]
	pub use crate::build::prelude::*;
	pub use crate::core::prelude::*;
	#[cfg(feature = "design")]
	pub use crate::design::prelude::*;
	#[cfg(feature = "dom")]
	pub use crate::dom::prelude::*;
	#[cfg(feature = "examples")]
	pub use crate::examples::prelude::*;
	#[cfg(feature = "flow")]
	pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	#[cfg(feature = "parse")]
	pub use crate::parse::prelude::*;
	// #[cfg(feature = "query")]
	// pub use crate::query::prelude::*;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	#[cfg(feature = "rsx")]
	pub use crate::rsx::prelude::*;
	#[cfg(feature = "sim")]
	pub use crate::sim::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
	pub use crate::utils::prelude::*;
	/// hack to fix bevy macros
	pub use bevy::ecs as bevy_ecs;
	pub use bevy::prelude::*;
	/// hack to fix bevy macros
	pub use bevy::reflect as bevy_reflect;
	// beet workflows make heavy use of `RunSystemOnce` to run systems
	pub use bevy::ecs::system::RunSystemOnce;
}
pub mod exports {
	#[cfg(feature = "build")]
	pub use crate::build::exports::*;
	pub use crate::core::exports::*;
	#[cfg(feature = "design")]
	pub use crate::design::exports::*;
	#[cfg(feature = "rsx")]
	pub use crate::rsx::exports::*;
	pub use crate::utils::exports::*;
	#[cfg(feature = "examples")]
	pub use beet_examples::exports::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::exports::*;
	#[cfg(feature = "net")]
	pub use beet_net::exports::*;
	#[cfg(feature = "sim")]
	pub use beet_sim::exports::*;
	#[cfg(feature = "spatial")]
	pub use beet_spatial::exports::*;
	pub use bevy;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
