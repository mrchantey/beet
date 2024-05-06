use super::*;
use std::borrow::Cow;

#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component, Default)]
pub struct RenderText(pub Cow<'static, str>);

impl RenderText {
	pub fn new(text: impl Into<Cow<'static, str>>) -> Self { Self(text.into()) }
}

impl Default for RenderText {
	fn default() -> Self { Self::new("🥕") }
}
