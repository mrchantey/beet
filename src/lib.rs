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
#[cfg(feature = "router")]
pub use beet_router as router;
#[cfg(feature = "thread")]
pub use beet_thread as thread;
#[cfg(feature = "ui")]
pub use beet_ui as ui;
// #[cfg(feature = "design")]
// pub use beet_design as design;
// #[cfg(feature = "dom")]
// pub use beet_dom as dom;
#[cfg(feature = "examples")]
pub use beet_examples as examples;
// #[cfg(feature = "flow")]
// pub use beet_flow as flow;
// #[cfg(feature = "ml")]
// pub use beet_ml as ml;
// #[cfg(feature = "parse")]
// pub use beet_parse as parse;
// #[cfg(feature = "rsx")]
// pub use beet_rsx as rsx;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;
#[rustfmt::skip]
pub mod prelude {
	#[cfg(feature = "action")]
	pub use crate::action::prelude::*;
	pub use crate::beet_plugins::*;
	pub use crate::core::prelude::*;
	#[cfg(feature = "infra")]
	pub use crate::infra::prelude::*;
	#[cfg(all(feature = "net",feature="json"))]
	pub use crate::net::prelude::TableStore;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "ui")]
	pub use crate::core::prelude::Pointer;
	#[cfg(feature = "ui")]
	pub use crate::ui::prelude::*;
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
	#[cfg(feature = "examples")]
	pub use crate::examples::prelude::*;
	// #[cfg(feature = "flow")]
	// pub use crate::flow::prelude::*;
	// #[cfg(feature = "ml")]
	// pub use crate::ml::prelude::*;
	// #[cfg(feature = "parse")]
	// pub use crate::parse::prelude::*;
	// #[cfg(feature = "rsx")]
	// pub use crate::rsx::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
	cfg_if! {
		if #[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]{
			pub use crate::ui::prelude::Justify;
			pub use crate::ui::prelude::Pointer;
		}
	}
	cfg_if! {
		// re-exports to disambiguate bevy ui
		// overlaps here are a feature not a bug,
		// we're aligned on layout design patterns, and not dependent on
		// the bevy_ui pixel rendering specificities
		if #[cfg(feature = "ui")]{
			pub use crate::ui::prelude::style::*;
			pub use crate::ui::prelude::style::Display;
			pub use crate::ui::prelude::style::JustifyContent;
			pub use crate::ui::prelude::style::FontWeight;
			pub use crate::ui::prelude::style::AlignSelf;
			pub use crate::ui::prelude::style::AlignItems;
			pub use crate::ui::prelude::style::AlignContent;
			pub use crate::ui::prelude::style::FlexWrap;
		}
	}
}

pub mod exports {
	pub use crate::core::exports::*;
	#[cfg(feature = "net")]
	pub use beet_net::exports::*;
	#[cfg(feature = "ui")]
	pub use beet_ui::exports::*;
	pub use bevy;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
