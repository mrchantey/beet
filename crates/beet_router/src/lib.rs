#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(more_qualified_paths, if_let_guard)]

// #[cfg(not(target_arch = "wasm32"))]
// pub mod build_ssg;
// pub mod client_islands;
// pub mod server_actions;
// pub mod types;

pub mod prelude {
	// pub use crate::app_cx;
	// #[cfg(not(target_arch = "wasm32"))]
	// pub use crate::build_ssg::*;
	// pub use crate::client_islands::*;
	// pub use crate::server_actions::*;
	// pub use crate::types::*;

	pub use sweet::prelude::HttpMethod;
}


pub mod exports {
	pub use http;
	pub use sweet::prelude::GlobFilter;
}

/// expose prelude and as beet for macros
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as router;
		pub use beet_rsx as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_rsx::prelude::*;
			pub use beet_common::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
			pub use beet_rsx::exports::*;
		}
	}
}

// #[cfg(any(test, feature = "_test_site"))]
// pub mod test_site;
