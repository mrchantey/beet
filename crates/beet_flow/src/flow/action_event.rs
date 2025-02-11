use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

pub trait ActionEvent: Event + Debug {
	fn action(&self) -> Entity;
	fn origin(&self) -> Entity;
}

impl<T: RunPayload> ActionEvent for OnRun<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


impl<T: ResultPayload> ActionEvent for OnResult<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


/// Collect all [OnRunAction] with a [Name]
#[cfg(test)]
pub fn collect_on_run(world: &mut World) -> impl Fn() -> Vec<String> {
	let func = sweet::prelude::mock_bucket();
	let func2 = func.clone();
	world.add_observer(move |ev: Trigger<OnRunAction>, query: Query<&Name>| {
		let action = if ev.action == Entity::PLACEHOLDER {
			ev.entity()
		} else {
			ev.action
		};
		if let Ok(name) = query.get(action) {
			func2.call(name.to_string());
		}
	});
	move || func.called.lock().unwrap().clone()
}

/// Collect all [OnResultAction] with a [Name]
#[cfg(test)]
pub fn collect_on_result(
	world: &mut World,
) -> impl Fn() -> Vec<(String, RunResult)> {
	let func = sweet::prelude::mock_bucket();
	let func2 = func.clone();
	world.add_observer(
		move |ev: Trigger<OnResultAction>, query: Query<&Name>| {
			if let Ok(name) = query.get(ev.action) {
				func2.call((name.to_string(), ev.payload.clone()));
			}
		},
	);
	move || func.called.lock().unwrap().clone()
}
