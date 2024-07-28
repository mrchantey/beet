// #![allow(unused, dead_code)]
pub mod components;
pub mod plugins;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
}

// i forget why i did this
pub mod beet {
	pub mod prelude {
		pub use beet_flow::prelude::*;
		pub use beet_ml::prelude::*;
		pub use beet_spatial::prelude::*;
	}
}
