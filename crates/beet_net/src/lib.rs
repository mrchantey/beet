#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard)]

mod http_utils;
mod object_storage;
mod transport;

pub mod prelude {
	pub use crate::http_utils::*;
	pub use crate::object_storage::*;

	// reexport common types
	pub use http::StatusCode;
	pub use http::header;
	pub use url::Url;

	pub use bevy::tasks::futures_lite::StreamExt;
}


pub mod exports {
	pub use bevy::tasks::futures_lite;
	pub use eventsource_stream;
	pub use http;
	pub use http_body_util;
	pub use url;
}
