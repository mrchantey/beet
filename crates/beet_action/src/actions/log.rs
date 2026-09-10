//! Debug logging leaf action.
use crate::prelude::*;
use beet_core::prelude::*;

/// Logs a message when called, then returns [`Outcome::PASS`].
///
/// With a `message` it logs that string, otherwise it names the caller: its
/// [`Name`] when it has one, else its entity id. Logs at info level via the
/// `log` crate, visible on every platform.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(Log::new("running..."));
/// ```
#[action]
#[derive(Debug, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Log(
	/// The message to log; absent names the caller instead.
	#[field]
	message: Option<SmolStr>,
	cx: ActionContext,
) -> Result<Outcome> {
	match message {
		Some(message) => info!("{message}"),
		None => match cx.caller.get(|name: &Name| name.to_string()).await {
			Ok(name) => info!("Running: {name}"),
			Err(_) => info!("Running: {}", cx.id()),
		},
	}
	Outcome::PASS.xok()
}

impl Log {
	/// Log a fixed message.
	pub fn new(message: impl Into<SmolStr>) -> Self {
		Self {
			message: Some(message.into()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn message_passes() {
		AsyncPlugin::world()
			.spawn(Log::new("hello"))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	/// Fields are read off the caller at call time, so a component removed out
	/// from under its own action fails loudly naming the type, never silently
	/// logging a default.
	#[beet_core::test]
	async fn missing_component_errors() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(Log::new("hello")).id();
		world.entity_mut(entity).remove::<Log>();
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Log");
	}

	#[beet_core::test]
	async fn name_passes() {
		AsyncPlugin::world()
			.spawn((Name::new("root"), Log::default()))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
