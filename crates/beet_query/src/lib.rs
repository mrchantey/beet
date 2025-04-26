#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]


// #[cfg(test)]
// pub mod testdb;
pub mod types;

pub mod prelude {
	// #[cfg(test)]
	// pub use crate::testdb::*;
	pub use crate::types::*;
}
