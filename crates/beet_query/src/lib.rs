#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]


// #[cfg(test)]
// pub mod testdb;
pub mod types;
pub mod query_builders;

// pub use beet_query_macros::*;

pub mod prelude {
	pub use beet_query_macros::*;
	// #[cfg(test)]
	// pub use crate::testdb::*;
	pub use crate::types::*;
	pub use crate::query_builders::*;
}


/// used for testing
pub mod as_beet {
	pub use crate::prelude::*;

	pub mod beet {
		pub use crate::prelude;
	}
}
