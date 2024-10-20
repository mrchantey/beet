// #![allow(unused, dead_code)]
pub mod components;
pub mod plugins;
pub mod emote_agent;
pub mod scenes;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
	pub use crate::emote_agent::*;
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
