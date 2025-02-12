use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

/// used internally for patterns common to
/// - [OnRun]
/// - [OnRunAction]
/// - [OnResult]
/// - [OnResultAction]
/// It is not exposed, primarily because calling
/// [Self::action()] or [Self::origin()] on
/// [OnRunAction] or [OnResultAction] would be incorrect,
/// instead resolve_action etc should be used.
pub trait ActionEvent: Event + Debug {
	/// Internal use only, use Trigger::resolve_action
	fn _action(&self) -> Entity;
	/// Internal use only, use Trigger::resolve_origin
	fn _origin(&self) -> Entity;
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
		if self._action() == Entity::PLACEHOLDER {
			if self.entity() == Entity::PLACEHOLDER {
				panic!("OnRunAction must either specify an action or be triggered on an action entity");
			} else {
				self.entity()
			}
		} else {
			self._action()
		}
	}

	/// Get the origin entity, or the entity that triggered the action.
	fn resolve_origin(&self) -> Entity {
		if self._origin() == Entity::PLACEHOLDER {
			self.resolve_action()
		} else {
			self._origin()
		}
	}
}


impl<T: RunPayload> ActionEvent for OnRun<T> {
	fn _action(&self) -> Entity { self.action }
	fn _origin(&self) -> Entity { self.origin }
}


impl<T: ResultPayload> ActionEvent for OnResult<T> {
	fn _action(&self) -> Entity { self.action }
	fn _origin(&self) -> Entity { self.origin }
}

impl<T: ResultPayload> ActionEvent for OnResultAction<T> {
	fn _action(&self) -> Entity { self.action }
	fn _origin(&self) -> Entity { self.origin }
}

/// Collect all [OnRunAction] with a [Name]
#[cfg(test)]
pub fn collect_on_run(world: &mut World) -> impl Fn() -> Vec<String> {
	let func = sweet::prelude::mock_bucket();
	let func2 = func.clone();
	world.add_observer(move |ev: Trigger<OnRunAction>, query: Query<&Name>| {
		let action = ev.resolve_action();
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
