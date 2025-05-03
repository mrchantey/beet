#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]


pub mod openai;



pub mod prelude {

	pub use crate::openai;
}
