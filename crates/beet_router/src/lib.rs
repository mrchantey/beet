#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
// #![feature(more_qualified_paths, if_let_guard, tuple_trait, unboxed_closures)]
pub mod prelude {
	pub const FOO: bool = true;
}


pub mod exports {
	pub use beet_utils::prelude::GlobFilter;
	pub use http;
}

/// Internal use only
pub mod as_beet {
	pub use beet::prelude::*;
	pub use bevy::prelude::*;
	pub mod beet {
		pub use crate as router;
		pub use beet_template as template;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_common::prelude::*;
			pub use beet_net::prelude::*;
			pub use beet_template::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_template::exports::*;
		}
	}
}

// #[cfg(any(test, feature = "_test_site"))]
// pub mod test_site;
