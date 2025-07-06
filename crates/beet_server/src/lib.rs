#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard, let_chains, stmt_expr_attributes)]
#![cfg_attr(
	feature = "nightly",
	feature(tuple_trait, fn_traits, unboxed_closures)
)]

// #![deny(missing_docs)]

mod app_router;
mod axum_utils;
#[cfg(feature = "lambda")]
mod lambda_utils;

pub mod prelude {
	pub use axum::Router;

	pub use crate::app_router::*;
	pub use crate::axum_utils::*;
	#[cfg(feature = "lambda")]
	pub use crate::lambda_utils::*;

	pub(crate) use internal::*;
	#[allow(unused_imports)]
	mod internal {
		pub use beet_rsx::as_beet::*;
		pub use beet_net::prelude::*;
		pub use beet_rsx::prelude::*;
		pub use beet_utils::prelude::*;
	}
}


pub mod exports {
	pub use axum;
}
