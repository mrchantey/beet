#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard, let_chains, stmt_expr_attributes)]
// #![deny(missing_docs)]

mod axum_utils;
mod beet_server;
#[cfg(feature = "lambda")]
mod lambda_utils;
mod rsx;

pub mod prelude {
	pub use crate::axum_utils::*;
	pub use crate::beet_server::*;
	#[cfg(feature = "lambda")]
	pub use crate::lambda_utils::*;
	pub use crate::rsx::*;
}


pub mod exports {
	pub use axum;
}
