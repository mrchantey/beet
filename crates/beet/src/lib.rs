pub use beet_ecs::*;
pub use beet_net::*;

pub mod base;


pub mod prelude {
	pub use crate::base::*;
	pub use beet_ecs::prelude::*;
	pub use beet_net::prelude::*;
}

pub mod exports {
	pub use beet_ecs::exports::*;
}
