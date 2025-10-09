use crate::prelude::*;
use beet_core::prelude::*;
use bevy::color::palettes::tailwind;
use std::borrow::Cow;



/// A plugin that logs lifecycle events for action entities.
/// If they have a [`Name`] that will be used instead of the entity id.
/// It emits [OnLogMessage] events, and also
/// will print to stdout if [`Self::log_to_stdout`] is true.
#[derive(Debug, Clone)]
pub struct DebugFlowPlugin {
	/// Log whenever [`Run`] is triggered.
	pub log_run: bool,
	/// Log whenever [`Running`] entities are updated.
	pub log_running: bool,
	/// Log whenever [`End`] is triggered.
	pub log_end: bool,
	/// Log all messages to stdout
	pub log_to_stdout: bool,
}
impl Default for DebugFlowPlugin {
	fn default() -> Self { Self::with_run() }
}

impl DebugFlowPlugin {
	/// Include:
	/// - [`log_on_run`](Self::log_on_run)
	/// - [`log_to_stdout`](Self::log_to_stdout)
	pub fn with_run() -> Self {
		Self {
			log_run: true,
			log_end: false,
			log_running: false,
			log_to_stdout: true,
		}
	}
	/// Include:
	/// - [`log_on_run`](Self::log_on_run)
	/// - [`log_on_run_result`](Self::log_on_result)
	/// - [`log_to_stdout`](Self::log_to_stdout)
	pub fn with_result() -> Self {
		Self {
			log_run: true,
			log_end: true,
			log_running: false,
			log_to_stdout: true,
		}
	}
	/// Include:
	/// - [`log_on_run`](Self::log_on_run)
	/// - [`log_running`](Self::log_running)
	/// - [`log_on_run_result`](Self::log_on_result)
	/// - [`log_to_stdout`](Self::log_to_stdout)
	pub fn with_all() -> Self {
		Self {
			log_run: true,
			log_running: true,
			log_end: true,
			log_to_stdout: true,
		}
	}
	/// Exclude all, add each manually and handle stdout
	/// ```rust
	///	# use beet_core::prelude::*;
	///	# use beet_flow::prelude::*;
	/// fn my_log_func(_ev: MessageReader<OnLogMessage>) {
	///
	/// }
	/// App::new()
	/// 	.add_plugins(DebugFlowPlugin::with_none())
	/// 	.add_systems(Update, my_log_func)
	/// 	.init_resource::<DebugOnRun>();
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

/// A helper event for logging messages.
/// This must use the [`MessageReader`] pattern instead of observers
/// because the 'stack' nature of observers results in a reverse order.
#[derive(Debug, Message)]
pub struct OnLogMessage {
	/// The message to log
	pub msg: Cow<'static, str>,
	/// The color of the message text
	pub color: Color,
}

impl OnLogMessage {
	/// The color of messages describing flow state.
	pub const FLOW_COLOR: Srgba = tailwind::NEUTRAL_200;
	/// The color of messages sent by the user.
	pub const USER_COLOR: Srgba = tailwind::CYAN_200;
	/// The color of messages sent by agents in the game.
	pub const GAME_COLOR: Srgba = tailwind::YELLOW_200;

	/// Create a new log message.
	pub fn new(
		msg: impl Into<Cow<'static, str>>,
		color: impl Into<Color>,
	) -> Self {
		Self {
			msg: msg.into(),
			color: color.into(),
		}
	}
	/// Create a new log message, with a [`Name`] query.
	pub fn new_with_query(
		entity: Entity,
		query: &Query<&Name>,
		prefix: &str,
		color: impl Into<Color>,
	) -> Self {
		Self::new_with_optional(entity, query.get(entity).ok(), prefix, color)
	}
	/// Create a new log message, with an [`Option<Name>`],
	/// falling back to the [`Entity`] if `None`.
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
	/// Immediately log to stdout, useful for initial messages
	pub fn and_log(self) -> Self {
		println!("{}", self.msg);
		self
	}
	/// Log to stdout
	pub fn log(&self) {
		println!("{}", self.msg);
	}
}

/// An event triggered to represent user input, useful for
/// retrieving user text input.
#[derive(Debug, Default, Clone, Deref, DerefMut, Event, Reflect)]
pub struct UserMessage(pub String);

impl UserMessage {
	/// Create a new user message.
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


/// Resource to enable logging for [log_on_run]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOnRun;
/// Resource to enable logging for [log_on_run_result]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOutcome;
/// Resource to enable logging for [log_running]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugRunning;
/// Resource to enable logging for [log_to_stdout]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugToStdOut;


// fn log_to_stdout(mut read: MessageReader<OnLogMessage>) {
// 	for msg in read.read() {
// 		println!("{}", msg.0);
// 	}
// }

fn log_on_run(
	ev: On<GetOutcome>,
	query: Query<&Name>,
	_m: When<Res<DebugOnRun>>,
	mut out: MessageWriter<OnLogMessage>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new_with_query(
		ev.action(),
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
		ev.action(),
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
