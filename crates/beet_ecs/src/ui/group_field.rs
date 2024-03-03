use super::*;
use std::fmt::Display;

#[derive(Clone)]
pub struct GroupField {
	pub display_name: String,
	pub children: Vec<FieldUi>,
}

impl GroupField {
	pub fn new(display_name: String, children: Vec<FieldUi>) -> Self {
		Self {
			display_name,
			children,
		}
	}
}
impl Display for GroupField {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GroupField")
			.field("display_name", &self.display_name)
			.finish()
	}
}
