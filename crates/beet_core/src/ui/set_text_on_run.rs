use beet_ecs::prelude::*;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;


#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
/// Sets the [`Text`] of all entities matching the query on run.
pub struct SetTextOnRun<F: QueryFilter> {
	pub value: Cow<'static, str>,
	pub section: usize,
	phantom: PhantomData<F>,
}

impl<F: QueryFilter> SetTextOnRun<F> {
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

fn set_text_on_run<F: 'static + Send + Sync + QueryFilter>(
	mut texts: Query<&mut Text, F>,
	query: Query<&SetTextOnRun<F>, Added<Running>>,
) {
	for set_text_on_run in query.iter() {
		for mut text in texts.iter_mut() {
			text.sections[set_text_on_run.section].value =
				set_text_on_run.value.to_string();
		}
	}
}

impl<F: QueryFilter> ActionMeta for SetTextOnRun<F> {
	fn category(&self) -> ActionCategory { ActionCategory::World }
}

impl<F: 'static + Send + Sync + QueryFilter> ActionSystems for SetTextOnRun<F> {
	fn systems() -> SystemConfigs { set_text_on_run::<F>.in_set(TickSet) }
}
