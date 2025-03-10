#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![deny(missing_docs)]

mod axum_utils;
#[cfg(feature = "lambda")]
mod lambda_utils;

pub use axum;

pub mod prelude {
	pub use crate::axum_utils::*;
	#[cfg(feature = "lambda")]
	pub use crate::lambda_utils::*;
}
