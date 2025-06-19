#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![doc = include_str!("../README.md")]
pub use beet_bevy as bevy;
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
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use beet_fs as fs;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "net")]
pub use beet_net as net;
#[cfg(feature = "parse")]
pub use beet_parse as parse;
#[cfg(feature = "query")]
pub use beet_query as query;
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use beet_server as server;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use beet_server_utils as server_utils;
#[cfg(feature = "sim")]
pub use beet_sim as sim;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;
#[cfg(feature = "template")]
pub use beet_template as template;
pub use beet_utils as utils;
pub use beet_utils::elog;
pub use beet_utils::log;
pub use beet_utils::noop;
#[cfg(feature = "web")]
pub use beet_web as web;
pub mod prelude {
	pub use crate::bevy::prelude::*;
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
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "parse")]
	pub use crate::parse::prelude::*;
	#[cfg(feature = "query")]
	pub use crate::query::prelude::*;
	#[cfg(feature = "router")]
	pub use crate::router::prelude::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::prelude::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server_utils::prelude::*;
	#[cfg(feature = "sim")]
	pub use crate::sim::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
	#[cfg(feature = "template")]
	pub use crate::template::prelude::*;
	pub use crate::utils::prelude::*;
	#[cfg(all(feature = "web", target_arch = "wasm32"))]
	pub use crate::web::prelude::*;
	pub use bevy::prelude::*;
}
pub mod exports {
	#[cfg(feature = "build")]
	pub use crate::build::exports::*;
	pub use crate::common::exports::*;
	#[cfg(feature = "design")]
	pub use crate::design::exports::*;
	#[cfg(feature = "fs")]
	pub use crate::fs::exports::*;
	#[cfg(feature = "router")]
	pub use crate::router::exports::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::exports::*;
	#[cfg(feature = "template")]
	pub use crate::template::exports::*;
	pub use crate::utils::exports::*;
	#[cfg(feature = "examples")]
	pub use beet_examples::exports::*;
	#[cfg(feature = "ml")]
	pub use beet_ml::exports::*;
	#[cfg(feature = "sim")]
	pub use beet_sim::exports::*;
	#[cfg(feature = "spatial")]
	pub use beet_spatial::exports::*;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
