use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

pub trait ActionEvent: Event + Debug {
	fn action(&self) -> Entity;
	fn origin(&self) -> Entity;

	fn origin_or_action(&self) -> Entity {
		if self.origin() == Entity::PLACEHOLDER {
			self.action()
		} else {
			self.origin()
		}
	}
}

#[extend::ext(name=ActionEventTriggerExt)]
pub impl<'w, T: ActionEvent> Trigger<'w, T> {
	/// Get the action entity, or the entity that triggered the action.
	/// Its assumed that if the action is [Entity::PLACEHOLDER] this event
	/// was called with ::local() and the action entity is the trigger entity.
	/// # Panics
	///
	/// If this trigger was called globally and the action entity is [Entity::PLACEHOLDER]
	fn resolve_action(&self) -> Entity {
		if self.action() == Entity::PLACEHOLDER {
			if self.entity() == Entity::PLACEHOLDER {
				panic!("OnRunAction must either specify an action or be triggered on an action entity");
			} else {
				self.entity()
			}
		} else {
			self.action()
		}
	}

	/// Get the origin entity, or the entity that triggered the action.
	fn resolve_origin(&self) -> Entity {
		if self.origin() == Entity::PLACEHOLDER {
			self.resolve_action()
		} else {
			self.origin()
		}
	}
}



impl<T: RunPayload> ActionEvent for OnRun<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}
impl<T: RunPayload> ActionEvent for OnRunAction<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


impl<T: ResultPayload> ActionEvent for OnResult<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}

impl<T: ResultPayload> ActionEvent for OnResultAction<T> {
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
			if let Ok(name) = query.get(ev.resolve_action()) {
				func2.call((name.to_string(), ev.payload.clone()));
			}
		},
	);
	move || func.called.lock().unwrap().clone()
}
