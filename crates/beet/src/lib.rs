pub use beet_core::*;
pub use beet_ecs::*;
// pub use beet_net::*;

pub mod prelude {
	extern crate beet_ecs as beet_ecs;
	
	pub use beet_core::prelude::*;
	pub use beet_ecs::prelude::*;
	// pub use beet_net::prelude::*;
}

pub mod exports {
	pub use beet_ecs::exports::*;
}
