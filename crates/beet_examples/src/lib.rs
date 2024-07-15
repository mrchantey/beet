// #![allow(unused, dead_code)]
pub mod serde_utils;
pub mod components;
pub mod plugins;
pub mod scenes;


pub mod prelude {
	pub use crate::components::*;
	pub use crate::plugins::*;
	pub use crate::scenes;
	pub use crate::serde_utils::*;
}

pub mod beet {
	pub mod prelude {
		pub use beet_core::prelude::*;
		pub use beet_ecs::prelude::*;
		pub use beet_ml::prelude::*;
		pub use beet_net::prelude::*;
	}
}
