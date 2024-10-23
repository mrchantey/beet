use beet_flow::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;


#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::World)]
#[systems(set_text_on_run::<F>.in_set(TickSet))]
/// Sets the [`Text`] of all entities with the filter component `F`.
pub struct SetTextOnRun<F: GenericActionComponent> {
	pub value: Cow<'static, str>,
	#[reflect(ignore)]
	phantom: PhantomData<F>,
}

impl<F: GenericActionComponent> SetTextOnRun<F> {
	pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
		Self {
			value: value.into(),
			phantom: PhantomData,
		}
	}
}

fn set_text_on_run<F: GenericActionComponent>(
	mut texts: Query<&mut Text, With<F>>,
	mut text_spans: Query<&mut TextSpan, With<F>>,
	query: Query<&SetTextOnRun<F>, Added<Running>>,
) {
	for set_text_on_run in query.iter() {
		for mut text in texts.iter_mut() {
			**text = set_text_on_run.value.to_string();
		}
		for mut text in text_spans.iter_mut() {
			**text = set_text_on_run.value.to_string();
		}
	}
}
