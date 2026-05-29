use beet_action::prelude::*;
use beet_core::prelude::*;
use alloc::borrow::Cow;
use core::marker::PhantomData;


/// Sets the [`Text`] of all entities with the filter component `F`
/// when this action runs, then passes.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[require(SetTextOnRunAction<F>)]
#[reflect(Component)]
pub struct SetTextOnRun<F: Component> {
	/// The text to set
	pub value: Cow<'static, str>,
	#[reflect(ignore)]
	phantom: PhantomData<F>,
}

impl<F: Component> SetTextOnRun<F> {
	/// Creates a new `SetTextOnRun` action.
	pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
		Self {
			value: value.into(),
			phantom: PhantomData,
		}
	}
}

/// Sets the text of all entities with the filter component `F`, then passes.
///
/// ## Errors
/// Errors if the caller has no [`SetTextOnRun`] component.
#[action(default)]
#[derive(Component)]
pub fn SetTextOnRunAction<F>(
	cx: In<ActionContext>,
	query: Query<&SetTextOnRun<F>>,
	mut texts: Query<&mut Text, With<F>>,
	mut text_spans: Query<&mut TextSpan, With<F>>,
) -> Result<Outcome>
where
	F: Component,
{
	let set_text_on_run = query.get(cx.id())?;
	for mut text in texts.iter_mut() {
		**text = set_text_on_run.value.to_string();
	}
	for mut text in text_spans.iter_mut() {
		**text = set_text_on_run.value.to_string();
	}
	Outcome::PASS.xok()
}
