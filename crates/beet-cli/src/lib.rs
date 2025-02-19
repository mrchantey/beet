#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod serve;
pub mod watch_templates;

pub mod prelude {
	pub use crate::serve::*;
	pub use crate::watch_templates::*;
}
