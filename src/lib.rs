#![doc = include_str!("../README.md")]
mod beet_plugins;

// #[cfg(feature = "build")]
// pub use beet_build as build;
#[cfg(feature = "action")]
pub use beet_action as action;
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
#[cfg(feature = "infra")]
pub use beet_infra as infra;
#[cfg(feature = "net")]
pub use beet_net as net;
#[cfg(feature = "node")]
pub use beet_node as node;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "thread")]
pub use beet_thread as thread;
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
#[rustfmt::skip]
pub mod prelude {
	#[cfg(feature = "action")]
	pub use crate::action::prelude::*;
	pub use crate::beet_plugins::*;
	pub use crate::core::prelude::*;
	#[cfg(feature = "infra")]
	pub use crate::infra::prelude::*;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::TableStore;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::*;
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	pub use beet_core::val;
	// #[cfg(feature = "build")]
	// pub use crate::build::prelude::*;
	#[cfg(feature = "thread")]
	pub use crate::thread::prelude::*;
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
	// 	/// NODE REEXPORTS
	#[cfg(all(
		feature = "node",
		feature = "tui",
		not(target_arch = "wasm32")
	))]
	pub use crate::node::prelude::Justify;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::Pointer;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::Button;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::AlignItems;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::FlexWrap;
	#[cfg(feature = "node")]
	pub use crate::node::prelude::Rect;
}

pub mod exports {
	pub use crate::core::exports::*;
	#[cfg(feature = "net")]
	pub use beet_net::exports::*;
	#[cfg(feature = "node")]
	pub use beet_node::exports::*;
	pub use bevy;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
