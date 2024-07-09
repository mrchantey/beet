use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;


#[derive(Debug, Clone, PartialEq, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::World)]
#[systems(set_text_on_run::<F>.in_set(TickSet))]
/// Sets the [`Text`] of all entities matching the query on run.
pub struct SetTextOnRun<F: GenericActionComponent> {
	pub value: Cow<'static, str>,
	pub section: usize,
	#[reflect(ignore)]
	phantom: PhantomData<F>,
}

impl<F: GenericActionComponent> SetTextOnRun<F> {
	pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
		Self {
			value: value.into(),
			section: 0,
			phantom: PhantomData,
		}
	}

	pub fn new_with_section(
		value: impl Into<Cow<'static, str>>,
		section: usize,
	) -> Self {
		Self {
			value: value.into(),
			section,
			phantom: PhantomData,
		}
	}

	pub fn with_section(mut self, section: usize) -> Self {
		self.section = section;
		self
	}
}

fn set_text_on_run<F: GenericActionComponent>(
	mut texts: Query<&mut Text, With<F>>,
	query: Query<&SetTextOnRun<F>, Added<Running>>,
) {
	for set_text_on_run in query.iter() {
		for mut text in texts.iter_mut() {
			text.sections[set_text_on_run.section].value =
				set_text_on_run.value.to_string();
		}
	}
}
