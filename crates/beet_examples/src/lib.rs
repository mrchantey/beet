#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod components;
pub mod plugins;
pub mod scenes;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
}


// because of cyclic deps we cant use beet directly
// so instead we make a pretend beet module
pub(crate) mod beet {
	// pub use beet_flow as flow;
	// pub use beet_ml as ml;
	// pub use beet_spatial as spatial;

	pub mod prelude {
		pub use beet_flow::prelude::*;
		pub use beet_ml::prelude::*;
		pub use beet_spatial::prelude::*;
	}
}
