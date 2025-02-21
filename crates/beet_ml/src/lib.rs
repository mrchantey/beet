#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(incomplete_features)]
#![doc = include_str!("../README.md")]
// feels a bit early for missing_docs
// #![deny(missing_docs)]
#![feature(let_chains, generic_const_exprs, const_trait_impl)]
pub mod frozen_lake;
pub mod language;
pub mod rl;
#[cfg(feature = "spatial")]
pub mod spatial;
#[cfg(test)]
pub mod test_utils;

pub mod rl_realtime;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	pub use crate::frozen_lake::*;
	pub use crate::language::*;
	pub use crate::rl::*;
	pub use crate::rl_realtime::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::*;
	#[cfg(test)]
	pub use crate::test_utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}
