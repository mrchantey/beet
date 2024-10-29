pub mod components;
pub mod plugins;
pub mod scenes;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
}


pub mod beet {
	pub use beet_flow as flow;
	pub use beet_ml as ml;
	pub use beet_spatial as spatial;

	pub mod prelude {
		pub use beet_flow::prelude::*;
		pub use beet_ml::prelude::*;
		pub use beet_spatial::prelude::*;
	}
}
