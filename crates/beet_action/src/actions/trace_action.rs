//! Debug tracing for the action call model.
//!
//! Replaces `beet_flow`'s `DebugFlowPlugin`: instead of observing
//! `GetOutcome`/`Outcome` lifecycle events, a [`TraceAction`] wraps an inner
//! action and logs on call entry and exit. [`OnLogMessage`]/[`UserMessage`]
//! are kept for log-UI consumers such as `beet_site`.
use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;
use std::fmt::Debug;

/// Message carrying a line of action-trace output for log UIs.
///
/// Uses the [`MessageReader`] pattern rather than observers so log lines stay
/// in emission order. Rendering concerns (colors) belong to the UI consumer.
#[derive(Debug, Message)]
pub struct OnLogMessage {
	/// The message text to display.
	pub msg: Cow<'static, str>,
}

impl OnLogMessage {
	/// Create a new log message.
	pub fn new(msg: impl Into<Cow<'static, str>>) -> Self {
		Self { msg: msg.into() }
	}
}

/// Event representing user text input, surfaced in the log stream.
#[derive(Debug, Default, Clone, Deref, DerefMut, Event, Reflect)]
pub struct UserMessage(pub String);

impl UserMessage {
	/// Create a new user message.
	pub fn new(message: impl Into<String>) -> Self { Self(message.into()) }
}

/// Middleware that logs on call entry and exit, then forwards.
///
/// Use via [`IntoWrapAction::wrap`]:
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(trace_action.wrap(Action::<(), Outcome>::new_fixed(Outcome::PASS)));
/// ```
pub async fn trace_action<In, Out>(
	input: In,
	next: Next<In, Out>,
) -> Result<Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync + Debug,
{
	let id = next.id();
	let name = next
		.world()
		.entity(id)
		.get(|name: &Name| name.to_string())
		.await
		.unwrap_or_else(|_| format!("{id}"));

	emit(next.world(), format!("OnRun: {name}")).await;
	let result = next.call(input).await;
	match &result {
		Ok(out) => emit(next.world(), format!("{name}: {out:?}")).await,
		Err(err) => emit(next.world(), format!("{name}: Err({err})")).await,
	}
	result
}

/// Log to stdout (cross-platform) and emit an [`OnLogMessage`].
async fn emit(world: &AsyncWorld, msg: String) {
	cross_log!("{msg}");
	world
		.with_then(move |world| {
			world.write_message(OnLogMessage::new(msg));
		})
		.await;
}

/// Registers [`OnLogMessage`] and logs [`UserMessage`] events.
#[derive(Debug, Default, Clone)]
pub struct DebugActionPlugin;

impl Plugin for DebugActionPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<OnLogMessage>()
			.register_type::<UserMessage>()
			.add_observer(log_user_message);
	}
}

fn log_user_message(
	ev: On<UserMessage>,
	mut out: MessageWriter<OnLogMessage>,
) {
	let msg = format!("User: {}", ev.event().0);
	cross_log!("{msg}");
	out.write(OnLogMessage::new(msg));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn traces_and_forwards() {
		AsyncPlugin::world()
			.spawn((Name::new("leaf"), trace_action.wrap(Action::<(), Outcome>::new_fixed(Outcome::PASS))))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
