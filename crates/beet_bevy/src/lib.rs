#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

pub mod extensions;
pub mod systems;
pub mod utilities;


pub mod prelude {
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::bundle_effect;

	pub use crate::extensions::*;
	pub use crate::systems::*;
	pub use crate::utilities::*;
}
