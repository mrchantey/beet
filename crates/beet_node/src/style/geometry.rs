use super::*;
use beet_core::prelude::*;



#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum Unit {
	Px(f32),
	Rem(f32),
	Percent(f32),
}

impl Unit {
	pub fn px(value: f32) -> Self { Self::Px(value) }
	pub fn rem(value: f32) -> Self { Self::Rem(value) }
	pub fn percent(value: f32) -> Self { Self::Percent(value) }
}
impl std::fmt::Display for Unit {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Px(value) => write!(f, "{}px", value),
			Self::Rem(value) => write!(f, "{}rem", value),
			Self::Percent(value) => write!(f, "{}%", value),
		}
	}
}

impl CssToken for Unit {
	fn to_css_value(&self) -> String { self.to_string() }
}
