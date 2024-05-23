use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Calls the given function when this behavior starts running.
#[derive(Deref, DerefMut, Component)]
pub struct CallOnRun(pub Box<dyn 'static + Send + Sync + FnMut()>);

impl CallOnRun {
	pub fn new<F: 'static + Send + Sync + FnMut()>(value: F) -> Self {
		Self(Box::new(value))
	}
}

impl ActionMeta for CallOnRun {
	fn category(&self) -> ActionCategory { ActionCategory::World }
}

impl ActionSystems for CallOnRun {
	fn systems() -> SystemConfigs { call_on_run.in_set(TickSet) }
}

fn call_on_run(mut query: Query<&mut CallOnRun, Added<Running>>) {
	for mut func in query.iter_mut() {
		func();
	}
}
