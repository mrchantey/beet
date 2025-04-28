#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(let_chains)]


// #[cfg(test)]
// pub mod testdb;
// pub mod query_builders;
#[cfg(feature = "limbo")]
pub mod limbo;
#[cfg(feature = "libsql")]
pub mod libsql;
pub mod types;
pub mod sea_query;
pub mod utils;
// pub use beet_query_macros::*;

pub mod prelude {
	#[cfg(feature = "limbo")]
	pub use crate::limbo::*;
	#[cfg(feature = "libsql")]
	pub use crate::libsql::*;
	pub use crate::sea_query::*;
	pub use beet_query_macros::*;
	// #[cfg(test)]
	// pub use crate::testdb::*;
	// pub use crate::query_builders::*;
	pub use crate::types::*;
	pub use crate::utils::*;
	pub use sea_query::IntoColumnDef;
}

pub mod exports {
	pub use sea_query;
}


/// used for testing
pub mod as_beet {
	pub use crate::prelude::*;


	pub mod beet {
		pub use crate::exports;
		pub use crate::prelude;
	}
}
