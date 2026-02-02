//! DOM representation and manipulation for beet applications.
//!
//! This crate provides the core types for representing HTML DOM structures
//! in the Bevy ECS, including elements, attributes, text nodes, and templates.
//!
//! # Modules
//!
//! - [`node`]: DOM node types (elements, text, attributes)
//! - [`utils`]: Utility types and helpers
//! - [`webdriver`]: WebDriver integration for testing (requires `webdriver` feature)

#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![allow(async_fn_in_trait)]
#![warn(missing_docs)]

mod node;
mod utils;
#[cfg(feature = "webdriver")]
mod webdriver;

/// Re-exports of commonly used DOM types and utilities.
pub mod prelude {

	pub use crate::node::*;
	pub use crate::utils::*;
	#[cfg(feature = "webdriver")]
	pub use crate::webdriver::*;
	#[cfg(feature = "tokens")]
	pub(crate) use beet_core::exports::*;
	#[cfg(feature = "tokens")]
	pub use beet_core::prelude::ToTokens;
}
