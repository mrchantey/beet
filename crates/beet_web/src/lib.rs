#![allow(async_fn_in_trait)]
#![feature(async_closure)]

pub mod dom;

pub mod prelude {
	pub use crate::dom::*;
}
