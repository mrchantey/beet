#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]

pub mod client_islands;
pub mod server_actions;

pub mod prelude {
	pub use crate::client_islands::*;
	pub use crate::server_actions::*;
}


/// Internal use only
pub mod as_beet {
	pub use beet::prelude::*;
	pub use bevy::prelude::*;
	pub mod beet {
		pub use crate as router;
		pub use beet_rsx as template;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_core::prelude::*;
			pub use beet_rsx::prelude::*;
		}
		pub mod exports {
			// pub use crate::exports::*;
			pub use beet_rsx::exports::*;
		}
	}
}

// #[cfg(any(test, feature = "_test_site"))]
// pub mod test_site;
