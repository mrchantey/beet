//! Debug logging plugin for action lifecycle events.
//!
//! This module provides [`DebugFlowPlugin`] which logs action execution
//! for debugging and visualization purposes. It emits [`OnLogMessage`]
//! events that can be consumed by UI systems.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::color::palettes::tailwind;
use std::borrow::Cow;



/// Plugin that logs lifecycle events for action entities.
///
/// Logs entity names (or IDs if unnamed) when actions run, complete, or are
/// actively running. Emits [`OnLogMessage`] events and optionally prints
/// to stdout.
///
/// # Configurations
///
/// - [`DebugFlowPlugin::with_run`]: Log only when actions start (default)
/// - [`DebugFlowPlugin::with_result`]: Log starts and completions
/// - [`DebugFlowPlugin::with_all`]: Log everything including running state
/// - [`DebugFlowPlugin::with_none`]: Manual configuration
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// App::new()
///     .add_plugins((
///         ControlFlowPlugin::default(),
///         DebugFlowPlugin::with_result(),
///     ));
/// ```
#[derive(Debug, Clone)]
pub struct DebugFlowPlugin {
	/// Log whenever [`GetOutcome`] is triggered.
	pub log_run: bool,
	/// Log whenever [`Running`] entities are updated.
	pub log_running: bool,
	/// Log whenever [`Outcome`] is triggered.
	pub log_end: bool,
	/// Print all messages to stdout.
	pub log_to_stdout: bool,
}
impl Default for DebugFlowPlugin {
	fn default() -> Self { Self::with_run() }
}

impl DebugFlowPlugin {
	/// Logs only when actions start running.
	///
	/// Includes:
	/// - Run events
	/// - stdout output
	pub fn with_run() -> Self {
		Self {
			log_run: true,
			log_end: false,
			log_running: false,
			log_to_stdout: true,
		}
	}

	/// Logs when actions start and when they complete with a result.
	///
	/// Includes:
	/// - Run events
	/// - Result events
	/// - stdout output
	pub fn with_result() -> Self {
		Self {
			log_run: true,
			log_end: true,
			log_running: false,
			log_to_stdout: true,
		}
	}

	/// Logs all lifecycle events.
	///
	/// Includes:
	/// - Run events
	/// - Running state updates
	/// - Result events
	/// - stdout output
	pub fn with_all() -> Self {
		Self {
			log_run: true,
			log_running: true,
			log_end: true,
			log_to_stdout: true,
		}
	}

	/// Disables all logging for manual configuration.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_flow::prelude::*;
	/// fn my_log_func(_ev: MessageReader<OnLogMessage>) {
	///     // Handle log messages...
	/// }
	///
	/// App::new()
	///     .add_plugins(DebugFlowPlugin::with_none())
	///     .add_systems(Update, my_log_func)
	///     .init_resource::<DebugOnRun>();
	/// ```
	pub fn with_none() -> Self {
		Self {
			log_run: false,
			log_running: false,
			log_end: false,
			log_to_stdout: false,
		}
	}
}

impl Plugin for DebugFlowPlugin {
	fn build(&self, app: &mut App) {
		// TODO when resolved: [Observers::run_if](https://github.com/bevyengine/bevy/issues/14195)
		app
			// maybe log_user_message belongs elsewhere
			.add_observer(log_user_message)
			.add_observer(log_on_run)
			.add_observer(log_on_run_result)
			.add_message::<OnLogMessage>()
			.add_systems(
				Update,
				// (
				log_running
					.run_if(resource_exists::<DebugRunning>)
					.in_set(PostTickSet),
				// log_to_stdout.run_if(resource_exists::<DebugToStdOut>),
				// )
				// .chain()
			);

		if self.log_run {
			app.init_resource::<DebugOnRun>();
		}

		if self.log_end {
			app.init_resource::<DebugOutcome>();
		}

		if self.log_running {
			app.init_resource::<DebugRunning>();
		}

		if self.log_to_stdout {
			app.init_resource::<DebugToStdOut>();
		}
	}
}

/// Message event for logging action lifecycle information.
///
/// This must use the [`MessageReader`] pattern instead of observers because
/// observers run in stack order which would reverse log output.
#[derive(Debug, Message)]
pub struct OnLogMessage {
	/// The message text to display.
	pub msg: Cow<'static, str>,
	/// The color for rendering this message.
	pub color: Color,
}

impl OnLogMessage {
	/// Color for control flow state messages.
	pub const FLOW_COLOR: Srgba = tailwind::NEUTRAL_200;
	/// Color for user input messages.
	pub const USER_COLOR: Srgba = tailwind::CYAN_200;
	/// Color for game/AI agent messages.
	pub const GAME_COLOR: Srgba = tailwind::YELLOW_200;

	/// Creates a new log message with the given text and color.
	pub fn new(
		msg: impl Into<Cow<'static, str>>,
		color: impl Into<Color>,
	) -> Self {
		Self {
			msg: msg.into(),
			color: color.into(),
		}
	}

	/// Creates a log message using an entity's [`Name`] if available.
	pub fn new_with_query(
		entity: Entity,
		query: &Query<&Name>,
		prefix: &str,
		color: impl Into<Color>,
	) -> Self {
		Self::new_with_optional(entity, query.get(entity).ok(), prefix, color)
	}

	/// Creates a log message with an optional [`Name`], falling back to entity ID.
	pub fn new_with_optional(
		entity: Entity,
		name: Option<&Name>,
		prefix: &str,
		color: impl Into<Color>,
	) -> Self {
		let msg = name
			.map(|n| format!("{prefix}: {n}"))
			.unwrap_or_else(|| format!("{prefix}: {entity}"));
		Self::new(msg, color)
	}

	/// Immediately logs to stdout and returns self for chaining.
	pub fn and_log(self) -> Self {
		println!("{}", self.msg);
		self
	}

	/// Logs the message to stdout.
	pub fn log(&self) {
		println!("{}", self.msg);
	}
}

/// Event representing user text input.
///
/// Useful for capturing and displaying user commands or chat messages
/// in the log stream.
#[derive(Debug, Default, Clone, Deref, DerefMut, Event, Reflect)]
pub struct UserMessage(pub String);

impl UserMessage {
	/// Creates a new user message with the given text.
	pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
}

fn log_user_message(
	trigger: On<UserMessage>,
	mut out: MessageWriter<OnLogMessage>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new(
		format!("User: {}", &trigger.event().0),
		OnLogMessage::USER_COLOR,
	);
	if stdout.is_some() {
		println!("{}", msg.msg);
	}
	out.write(msg);
}


/// Resource that enables logging for [`GetOutcome`] events.
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOnRun;

/// Resource that enables logging for [`Outcome`] events.
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOutcome;

/// Resource that enables logging for [`Running`] state updates.
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugRunning;

/// Resource that enables stdout output for log messages.
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugToStdOut;


fn log_on_run(
	ev: On<GetOutcome>,
	query: Query<&Name>,
	_m: When<Res<DebugOnRun>>,
	mut out: MessageWriter<OnLogMessage>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new_with_query(
		ev.target(),
		&query,
		"OnRun",
		OnLogMessage::FLOW_COLOR,
	);
	if stdout.is_some() {
		msg.log();
	}
	out.write(msg);
}


fn log_on_run_result(
	ev: On<Outcome>,
	query: Query<&Name>,
	mut out: MessageWriter<OnLogMessage>,
	_m: When<Res<DebugOutcome>>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new_with_query(
		ev.target(),
		&query,
		&format!("{:?}", &ev.event()),
		OnLogMessage::FLOW_COLOR,
	);
	if stdout.is_some() {
		msg.log();
	}
	out.write(msg);
}

fn log_running(
	mut out: MessageWriter<OnLogMessage>,
	query: Populated<(Entity, Option<&Name>), With<Running>>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	for (entity, name) in query.iter() {
		let name = name
			.map(|n| n.to_string())
			.unwrap_or_else(|| entity.to_string());
		let msg = OnLogMessage::new(
			format!("Running: {}", name),
			OnLogMessage::FLOW_COLOR,
		);
		if stdout.is_some() {
			msg.log();
		}
		out.write(msg);
	}
}
