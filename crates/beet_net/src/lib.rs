#![feature(
	fn_traits,
	async_fn_traits,
	async_closure,
	unboxed_closures,
	never_type
)]
pub mod pubsub;
pub mod relay;
pub mod topic;
pub mod utils;


pub mod prelude {
	pub use crate::pubsub::*;
	pub use crate::relay::*;
	pub use crate::topic::*;
	pub use crate::utils::*;
}
