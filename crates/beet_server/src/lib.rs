#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

mod app_router;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod axum_utils;
#[cfg(feature = "lambda")]
mod lambda_utils;

pub mod prelude {
	pub use http::StatusCode;

	pub use crate::app_router::*;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::axum_utils::*;
	#[cfg(feature = "lambda")]
	pub use crate::lambda_utils::*;

	pub(crate) use internal::*;
	#[allow(unused_imports)]
	mod internal {
		pub use beet_core::prelude::*;
		pub use beet_rsx::as_beet::*;
		pub use beet_rsx::prelude::*;
		pub use beet_utils::prelude::*;
	}
}
