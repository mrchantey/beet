#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
pub mod emote_agent;
pub mod plugins;
pub mod scenes;
pub mod causal_loop;


pub mod prelude {
	pub use crate::emote_agent::*;
	pub use crate::plugins::*;
	pub use crate::scenes::*;
	pub use crate::causal_loop::*;
}
