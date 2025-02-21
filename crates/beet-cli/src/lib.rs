#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(exit_status_error)]

pub mod serve_html;
pub mod watch_templates;

pub mod prelude {
	pub use crate::serve_html::*;
	pub use crate::watch_templates::*;
}
