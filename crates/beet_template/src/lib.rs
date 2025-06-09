#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(more_qualified_paths, let_chains)]
// #![deny(missing_docs)]
//!
pub use beet_template_macros::*;
pub mod html;
pub mod templating;
pub mod types;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[rustfmt::skip]
pub mod prelude {
	pub use beet_template_macros::*;
	pub use crate::html::*;
	pub use crate::types::*;
	pub use crate::templating::*;
	// #[cfg(target_arch = "wasm32")]
	// pub use crate::wasm::*;
}

pub mod exports {
	pub use anyhow;
}

// rsx macros expect 'beet'
// so import this
// `use beet_template::as_beet::*;`
// only for internal examples
// #[cfg(debug_assertions)]
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_common::prelude::*;
			pub use sweet::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
		}
	}
}
