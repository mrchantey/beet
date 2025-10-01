use beet_flow::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;


/// Sets the [`Text`] of all entities with the filter component `F`.
/// Be sure to add the [`set_text_on_run_plugin`] to your app.
#[action(set_text_on_run::<F>)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
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

fn set_text_on_run<F: Component>(
	ev: On<Run>,
	query: Query<&SetTextOnRun<F>, Added<Running>>,
	mut texts: Query<&mut Text, With<F>>,
	mut text_spans: Query<&mut TextSpan, With<F>>,
) {
	let set_text_on_run = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	for mut text in texts.iter_mut() {
		**text = set_text_on_run.value.to_string();
	}
	for mut text in text_spans.iter_mut() {
		**text = set_text_on_run.value.to_string();
	}
}
