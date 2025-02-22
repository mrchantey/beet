use super::PostTickSet;
use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// A plugin that logs lifecycle events for action entities.
/// If they have a [`Name`] that will be used instead of the entity id.
/// It emits [OnLogMessage] events, and also
/// will print to stdout if [`Self::log_to_stdout`] is true.
#[derive(Debug, Clone)]
pub struct BeetDebugPlugin {
	/// Log whenever [OnRunAction] is triggered.
	pub log_on_run: bool,
	/// Log whenever [Running] entities are updated.
	pub log_running: bool,
	/// Log whenever [OnResultAction] is triggered.
	pub log_on_result: bool,
	/// Log all messages to stdout
	pub log_to_stdout: bool,
}
impl Default for BeetDebugPlugin {
	fn default() -> Self { Self::with_run() }
}

impl BeetDebugPlugin {
	/// Include:
	/// - [`log_on_run`](Self::log_on_run)
	/// - [`log_to_stdout`](Self::log_to_stdout)
	pub fn with_run() -> Self {
		Self {
			log_on_run: true,
			log_on_result: false,
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
			log_on_run: true,
			log_on_result: true,
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
			log_on_run: true,
			log_running: true,
			log_on_result: true,
			log_to_stdout: true,
		}
	}
	/// Exclude all, add each manually and handle stdout
	/// ```rust
	///	# use bevy::prelude::*;
	///	# use beet_flow::prelude::*;
	/// fn my_log_func(_ev: EventReader<OnLogMessage>) {
	///
	/// }
	/// App::new()
	/// 	.add_plugins(BeetDebugPlugin::with_none())
	/// 	.add_systems(Update, my_log_func)
	/// 	.init_resource::<DebugOnRun>();
	/// ```
	pub fn with_none() -> Self {
		Self {
			log_on_run: false,
			log_running: false,
			log_on_result: false,
			log_to_stdout: false,
		}
	}
}

impl Plugin for BeetDebugPlugin {
	fn build(&self, app: &mut App) {
		// TODO when resolved: [Observers::run_if](https://github.com/bevyengine/bevy/issues/14195)
		app
			// maybe log_user_message belongs elsewhere
			.add_observer(log_user_message)
			.add_observer(log_on_run.never_param_warn())
			.add_observer(log_on_run_result.never_param_warn())
			.add_event::<OnLogMessage>()
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

		if self.log_on_run {
			app.init_resource::<DebugOnRun>();
		}

		if self.log_on_result {
			app.init_resource::<DebugOnResult>();
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
/// This must use the [`EventReader`] pattern instead of observers
/// because the 'stack' nature of observers results in a reverse order.
#[derive(Debug, Event, Deref)]
pub struct OnLogMessage(pub Cow<'static, str>);

impl OnLogMessage {
	/// Create a new log message.
	pub fn new(msg: impl Into<Cow<'static, str>>) -> Self { Self(msg.into()) }
	/// Create a new log message, with a [`Name`] query.
	pub fn new_with_query(
		entity: Entity,
		query: &Query<&Name>,
		prefix: &str,
	) -> Self {
		Self::new_with_optional(entity, query.get(entity).ok(), prefix)
	}
	/// Create a new log message, with an [`Option<Name>`],
	/// falling back to the [`Entity`] if `None`.
	pub fn new_with_optional(
		entity: Entity,
		name: Option<&Name>,
		prefix: &str,
	) -> Self {
		let msg = name
			.map(|n| format!("{prefix}: {n}"))
			.unwrap_or_else(|| format!("{prefix}: {entity}"));
		Self(msg.into())
	}
	/// Immediately log to stdout, useful for initial messages
	pub fn and_log(self)->Self {
		println!("{}", self.0);
		self
	}
}

/// An event triggered to represent user input, useful for
/// retrieving user text input.
#[derive(Debug, Default, Clone, Deref, DerefMut, Event, Reflect)]
pub struct OnUserMessage(pub String);

impl OnUserMessage {
	/// Create a new user message.
	pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
}

fn log_user_message(
	trigger: Trigger<OnUserMessage>,
	mut out: EventWriter<OnLogMessage>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new(format!("User: {}", &trigger.event().0));
	if stdout.is_some() {
		println!("{}", msg.0);
	}
	out.send(msg);
}


/// Resource to enable logging for [log_on_run]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOnRun;
/// Resource to enable logging for [log_on_run_result]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugOnResult;
/// Resource to enable logging for [log_running]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugRunning;
/// Resource to enable logging for [log_to_stdout]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugToStdOut;


// fn log_to_stdout(mut read: EventReader<OnLogMessage>) {
// 	for msg in read.read() {
// 		println!("{}", msg.0);
// 	}
// }

fn log_on_run(
	ev: Trigger<OnRunAction>,
	query: Query<&Name>,
	_m: Res<DebugOnRun>,
	mut out: EventWriter<OnLogMessage>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg =
		OnLogMessage::new_with_query(ev.resolve_action(), &query, "OnRun");
	if stdout.is_some() {
		println!("{}", msg.0);
	}
	out.send(msg);
}


fn log_on_run_result(
	ev: Trigger<OnResultAction>,
	query: Query<&Name>,
	mut out: EventWriter<OnLogMessage>,
	_m: Res<DebugOnResult>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	let msg = OnLogMessage::new_with_query(
		ev.resolve_action(),
		&query,
		&format!("{:?}", &ev.payload),
	);
	if stdout.is_some() {
		println!("{}", msg.0);
	}
	out.send(msg);
}

fn log_running(
	mut out: EventWriter<OnLogMessage>,
	query: Populated<(Entity, Option<&Name>), With<Running>>,
	stdout: Option<Res<DebugToStdOut>>,
) {
	for (entity, name) in query.iter() {
		let name = name
			.map(|n| n.to_string())
			.unwrap_or_else(|| entity.to_string());
		let msg = OnLogMessage::new(format!("Running: {}", name));
		if stdout.is_some() {
			println!("{}", msg.0);
		}
		out.send(msg);
	}
}
