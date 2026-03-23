#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![doc = include_str!("../README.md")]
mod beet_plugins;

// #[cfg(feature = "build")]
// pub use beet_build as build;
pub use beet_core as core;
pub use beet_core::cross_log;
pub use beet_core::cross_log_error;
pub use beet_core::main;
#[cfg(feature = "testing")]
pub use beet_core::test;
#[cfg(feature = "testing")]
pub use beet_core::test_runner;
#[cfg(feature = "testing")]
pub use beet_core::testing;
#[cfg(feature = "net")]
pub use beet_net as net;
#[cfg(feature = "node")]
pub use beet_node as node;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "social")]
pub use beet_social as social;
#[cfg(feature = "stack")]
pub use beet_stack as stack;
#[cfg(feature = "tool")]
pub use beet_tool as tool;
// #[cfg(feature = "design")]
// pub use beet_design as design;
// #[cfg(feature = "dom")]
// pub use beet_dom as dom;
// #[cfg(feature = "examples")]
// pub use beet_examples as examples;
// #[cfg(feature = "flow")]
// pub use beet_flow as flow;
// #[cfg(feature = "ml")]
// pub use beet_ml as ml;
// #[cfg(feature = "parse")]
// pub use beet_parse as parse;
// #[cfg(feature = "rsx")]
// pub use beet_rsx as rsx;
// #[cfg(feature = "spatial")]
// pub use beet_spatial as spatial;
pub mod prelude {
	pub use crate::beet_plugins::*;
	pub use crate::core::prelude::*;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::*;
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	#[cfg(feature = "stack")]
	pub use crate::stack::prelude::*;
	#[cfg(feature = "tool")]
	pub use crate::tool::prelude::*;
	// #[cfg(feature = "build")]
	// pub use crate::build::prelude::*;
	#[cfg(feature = "social")]
	pub use crate::social::prelude::*;
	// #[cfg(feature = "design")]
	// pub use crate::design::prelude::*;
	// #[cfg(feature = "dom")]
	// pub use crate::dom::prelude::*;
	// #[cfg(feature = "examples")]
	// pub use crate::examples::prelude::*;
	// #[cfg(feature = "flow")]
	// pub use crate::flow::prelude::*;
	// #[cfg(feature = "ml")]
	// pub use crate::ml::prelude::*;
	// #[cfg(feature = "parse")]
	// pub use crate::parse::prelude::*;
	// #[cfg(feature = "rsx")]
	// pub use crate::rsx::prelude::*;
	// #[cfg(feature = "spatial")]
	// pub use crate::spatial::prelude::*;
}
pub mod exports {
	pub use crate::core::exports::*;
	#[cfg(feature = "net")]
	pub use beet_net::exports::*;
	#[cfg(feature = "node")]
	#[allow(unused)]
	pub use beet_node::exports::*;
	pub use bevy;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
