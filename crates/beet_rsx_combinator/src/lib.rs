#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

mod combinator_parser;
mod parse_attributes;
mod parse_attributes_types;
mod parse_children;
mod parse_children_types;
mod parse_elements;
mod parse_elements_types;
mod parse_external;
mod parse_external_types;
mod parse_html;
mod parse_js;
mod parse_js_types;
mod parse_misc;
mod parse_rsx;
mod parse_rust;
mod parse_rust_types;

#[cfg(test)]
mod test_helpers;
mod to_html;

pub mod types {
	pub use crate::parse_attributes_types::*;
	pub use crate::parse_children_types::*;
	pub use crate::parse_elements_types::*;
	pub use crate::parse_external_types::*;
	pub use crate::parse_js_types::*;
	pub use crate::parse_rust_types::*;
}

pub mod prelude {
	pub use crate::combinator_parser::*;
	pub use crate::parse_attributes::*;
	pub use crate::parse_children::*;
	pub use crate::parse_elements::*;
	pub use crate::parse_external::*;
	pub use crate::parse_html::*;
	pub use crate::parse_js::*;
	pub use crate::parse_misc::*;
	pub use crate::parse_rsx::*;
	pub use crate::parse_rust::*;
	pub use crate::to_html::*;
	pub use crate::types::*;
}
