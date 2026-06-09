#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// BeetPlugins pulls in std-only infrastructure (tracing-subscriber, package
// config), so the whole module is std-gated.
#[cfg(feature = "std")]
mod beet_plugins;

// #[cfg(feature = "build")]
// pub use beet_build as build;
#[cfg(feature = "action")]
pub use beet_action as action;
pub use beet_core as core;
pub use beet_core::cross_log;
pub use beet_core::cross_log_error;
#[cfg(feature = "custom_test_frameworks")]
pub use beet_core::libtest_runner;
pub use beet_core::main;
#[cfg(feature = "testing")]
pub use beet_core::test;
#[cfg(feature = "testing")]
pub use beet_core::test_main;
#[cfg(feature = "testing")]
pub use beet_core::test_runner;
#[cfg(feature = "testing")]
pub use beet_core::testing;
#[cfg(feature = "examples")]
pub use beet_examples as examples;
#[cfg(feature = "infra")]
pub use beet_infra as infra;
#[cfg(feature = "net")]
pub use beet_net as net;
#[cfg(any(feature = "router", feature = "router_embedded"))]
pub use beet_router as router;
#[cfg(feature = "thread")]
pub use beet_thread as thread;
#[cfg(feature = "ui")]
pub use beet_ui as ui;
// #[cfg(feature = "flow")]
// pub use beet_flow as flow;
#[cfg(feature = "ml")]
pub use beet_ml as ml;
#[cfg(feature = "spatial")]
pub use beet_spatial as spatial;
#[rustfmt::skip]
pub mod prelude {
	#[cfg(feature = "action")]
	pub use crate::action::prelude::*;
	#[cfg(feature = "std")]
	pub use crate::beet_plugins::*;
	pub use crate::core::prelude::*;
	#[cfg(feature = "infra")]
	pub use crate::infra::prelude::*;
	// `TableStore` overlaps: `beet_net`'s store struct (json+std) vs
	// `beet_thread`'s trait. `thread` transitively compiles beet_net with
	// json+std, so cover it too; prefer the net store.
	#[cfg(all(
		feature = "net",
		any(all(feature = "json", feature = "std"), feature = "thread")
	))]
	pub use crate::net::prelude::TableStore;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(feature = "ui")]
	pub use crate::ui::prelude::*;
	#[cfg(any(feature = "router", feature = "router_embedded"))]
	pub use crate::router::prelude::*;
	pub use beet_core::val;
	// #[cfg(feature = "build")]
	// pub use crate::build::prelude::*;
	#[cfg(feature = "thread")]
	pub use crate::thread::prelude::*;
	// #[cfg(feature = "design")]
	// pub use crate::design::prelude::*;
	#[cfg(feature = "examples")]
	pub use crate::examples::prelude::*;
	// #[cfg(feature = "flow")]
	// pub use crate::flow::prelude::*;
	#[cfg(feature = "ml")]
	pub use crate::ml::prelude::*;
	// #[cfg(feature = "parse")]
	// pub use crate::parse::prelude::*;
	// #[cfg(feature = "rsx")]
	// pub use crate::rsx::prelude::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::prelude::*;
	cfg_if! {
		if #[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]{
			pub use crate::ui::prelude::Justify;
		}
	}
	cfg_if! {
		// re-exports to disambiguate bevy ui
		// overlaps here are a feature not a bug,
		// we're aligned on layout design patterns, and not dependent on
		// the bevy_ui pixel rendering specificities
		if #[cfg(feature = "ui")]{
			pub use beet_core::prelude::Transform;
			pub use crate::ui::prelude::Reset;
			pub use crate::ui::prelude::style::*;
			pub use crate::ui::prelude::style::Display;
			pub use crate::ui::prelude::style::JustifyContent;
			pub use crate::ui::prelude::style::FontWeight;
			pub use crate::ui::prelude::style::FontStyle;
			pub use crate::ui::prelude::style::AlignSelf;
			pub use crate::ui::prelude::style::AlignItems;
			pub use crate::ui::prelude::style::AlignContent;
			pub use crate::ui::prelude::style::Visibility;
			pub use crate::ui::prelude::style::FlexWrap;
			pub use crate::ui::prelude::Pointer;
		}
	}
	cfg_if! {
		// widgets live behind beet_ui's `scene` feature
		if #[cfg(feature = "scene")]{
			pub use crate::ui::prelude::Button;
			pub use crate::ui::prelude::SidebarNode;
		}
	}
	// ui's widget `Header`/`Table` types overlap with `beet_net`'s `Header`
	// trait and `beet_thread`'s `Table` trait. `thread` transitively compiles
	// beet_ui with `scene` (via `beet_router`), so cover it alongside `scene`;
	// prefer the ui widgets.
	#[cfg(all(feature = "ui", any(feature = "scene", feature = "thread")))]
	pub use crate::ui::prelude::Header;
	#[cfg(all(feature = "ui", any(feature = "scene", feature = "thread")))]
	pub use crate::ui::prelude::Table;
}

pub mod exports {
	pub use crate::core::exports::*;
	#[cfg(feature = "action")]
	#[allow(unused)]
	pub use beet_action::exports::*;
	#[cfg(feature = "net")]
	#[allow(unused)]
	pub use beet_net::exports::*;
	#[cfg(feature = "ui")]
	#[allow(unused)]
	pub use beet_ui::exports::*;
	pub use bevy;
}
#[cfg(test)]
mod test {
	#[test]
	fn compiles() {}
}
