use alloc::borrow::Cow;
use beet_action::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;

/// Sets the [`Text`] of all entities with the filter component `F`
/// when this action runs, then passes.
#[action(plain_meta)]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub fn SetTextOnRun<F>(
	/// The text to set
	#[field]
	value: Cow<'static, str>,
	_cx: In<ActionContext>,
	mut texts: Query<&mut Text, With<F>>,
	mut text_spans: Query<&mut TextSpan, With<F>>,
) -> Result<Outcome>
where
	F: Component,
{
	for mut text in texts.iter_mut() {
		**text = value.to_string();
	}
	for mut text in text_spans.iter_mut() {
		**text = value.to_string();
	}
	Outcome::PASS.xok()
}

impl<F: Component> SetTextOnRun<F> {
	/// Creates a new `SetTextOnRun` action.
	pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
		Self {
			value: value.into(),
			_marker: PhantomData,
		}
	}
}
