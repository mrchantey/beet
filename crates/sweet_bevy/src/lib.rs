#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]

pub mod extensions;
pub mod systems;
pub mod utilities;


pub mod prelude {
	pub use crate::extensions::*;
	pub use crate::systems::*;
	pub use crate::utilities::*;
}
