use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

/// Common functions for [`OnRunAction`] and [`OnResultAction`] triggers.
/// This crate is private, primarily because calling
/// [ActionEvent::action] or [ActionEvent::origin] is incorrect,
/// ie when created via [OnRunAction::local] etc, the action will
/// be [Entity::PLACEHOLDER].
/// Instead the extensions on [Trigger] should be used:
/// - [ActionEventTriggerExt::resolve_action]
/// - [ActionEventTriggerExt::resolve_origin]
pub(crate) trait ActionEvent {
	/// Internal use only, use Trigger::resolve_action
	fn action(&self) -> Entity;
	/// Internal use only, use Trigger::resolve_origin
	fn origin(&self) -> Entity;
}
/// Common functions for [ActionEvent] triggers.
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
/// Common functions for [`OnRun`] and [`OnResult`] triggers.
pub trait ObserverEvent: Event + Debug {
	/// Get the action entity for this event.
	fn action(&self) -> Entity;
	/// Get the origin entity for this event.
	fn origin(&self) -> Entity;
}

impl<T: RunPayload> ObserverEvent for OnRun<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


impl<T: ResultPayload> ObserverEvent for OnResult<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}

/// Collect all [OnRunAction] with a [Name]
#[cfg(test)]
pub fn collect_on_run(world: &mut World) -> impl Fn() -> Vec<String> {
	let func = sweet::prelude::mock_bucket();
	let func2 = func.clone();
	world.add_observer(move |ev: Trigger<OnRunAction>, query: Query<&Name>| {
		let action = ev.resolve_action();
		let name = if let Ok(name) = query.get(action) {
			name.to_string()
		} else {
			"".to_string()
		};
		func2.call(name);
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
			let action = ev.resolve_action();
			let name = if let Ok(name) = query.get(action) {
				name.to_string()
			} else {
				"".to_string()
			};
			func2.call((name, ev.payload.clone()));
		},
	);
	move || func.called.lock().unwrap().clone()
}
