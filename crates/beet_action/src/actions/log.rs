//! Debug logging leaf action.
use crate::prelude::*;
use alloc::borrow::Cow;
use beet_core::prelude::*;

/// Logs a message when called, then returns [`Outcome::PASS`].
///
/// Folds the old `LogOnRun` and `LogNameOnRun` into a single action:
/// [`Log::Message`] logs a fixed string, [`Log::Name`] logs the caller's
/// [`Name`]. Uses [`cross_log!`] so output is visible on wasm.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(Log::new("running..."));
/// ```
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[require(LogAction)]
#[reflect(Component)]
pub enum Log {
	/// Log a fixed message.
	Message(Cow<'static, str>),
	/// Log the caller's [`Name`].
	Name,
}

impl Default for Log {
	fn default() -> Self { Self::Name }
}

impl Log {
	/// Create a [`Log::Message`] from anything string-like.
	pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
		Self::Message(message.into())
	}
}

/// Logs per the [`Log`] component, then passes.
///
/// ## Errors
/// Errors if the caller has no [`Log`] component.
#[action(default)]
#[derive(Component)]
pub async fn LogAction(cx: ActionContext) -> Result<Outcome> {
	match cx.caller.get_cloned::<Log>().await? {
		Log::Message(message) => cross_log!("{message}"),
		Log::Name => {
			let name = cx
				.caller
				.get(|name: &Name| name.to_string())
				.await
				.unwrap_or_else(|_| "<unnamed>".to_string());
			cross_log!("Running: {name}");
		}
	}
	Outcome::PASS.xok()
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

	#[beet_core::test]
	async fn name_passes() {
		AsyncPlugin::world()
			.spawn((Name::new("root"), Log::Name))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
