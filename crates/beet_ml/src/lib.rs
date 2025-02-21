#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(incomplete_features)]
#![doc = include_str!("../README.md")]
// feels a bit early for missing_docs
// #![deny(missing_docs)]
#![feature(let_chains, generic_const_exprs, const_trait_impl)]

#[cfg(feature = "bevy_default")]
pub mod frozen_lake;
pub mod language;
#[cfg(feature = "bevy_default")]
pub mod rl;
#[cfg(feature = "bevy_default")]
pub mod rl_realtime;
#[cfg(feature = "spatial")]
pub mod spatial;
#[cfg(test)]
pub mod test_utils;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	#[cfg(feature = "bevy_default")]
	pub use crate::frozen_lake::*;
	pub use crate::language::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::rl::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::rl_realtime::*;
	#[cfg(feature = "spatial")]
	pub use crate::spatial::*;
	#[cfg(test)]
	pub use crate::test_utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}
