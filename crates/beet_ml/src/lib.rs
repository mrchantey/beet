#![allow(incomplete_features)]
#![feature(let_chains, generic_const_exprs, const_trait_impl)]
pub mod environments;
pub mod ml_plugin;
pub mod language;
pub mod reinforcement;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	pub use crate::environments::frozen_lake::*;
	pub use crate::environments::*;
	pub use crate::ml_plugin::selectors::*;
	pub use crate::ml_plugin::*;
	pub use crate::language::*;
	pub use crate::reinforcement::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}
