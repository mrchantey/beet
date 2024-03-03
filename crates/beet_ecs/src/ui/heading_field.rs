use std::fmt::Display;


#[derive(Clone, PartialEq)]
pub struct HeadingField {
	pub text: String,
	pub size: f32,
}

impl HeadingField {
	pub fn new(text: String) -> Self { Self { text, size: 0.2 } }
}
impl Display for HeadingField {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("HeadingField")
			.field("text", &self.text)
			.field("size", &self.size)
			.finish()
	}
}
