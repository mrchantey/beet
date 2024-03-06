#![allow(async_fn_in_trait)]
#![feature(async_closure)]

pub mod dom;
pub mod app;

pub mod prelude {
	pub use crate::app::*;
	pub use crate::dom::*;
}
