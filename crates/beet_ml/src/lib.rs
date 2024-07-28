#![allow(incomplete_features)]
#![feature(let_chains, generic_const_exprs, const_trait_impl)]
pub mod environments;
pub mod language;
pub mod rl;
pub mod rl_realtime;
#[cfg(feature = "render")]
pub mod scenes;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
	pub use crate::environments::frozen_lake::*;
	pub use crate::environments::*;
	pub use crate::language::selectors::*;
	pub use crate::language::*;
	pub use crate::rl::*;
	pub use crate::rl_realtime::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}
