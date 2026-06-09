//! Scheduling for the post-parse pipeline.
//!
//! [`PostParseTree`] is the single schedule where a parsed entity tree is
//! styled, decorated, laid out, and painted. Every render path runs it
//! on demand: one-off parsers (eg
//! [`MarkdownParser`](crate::prelude::MarkdownParser)) and the charcell
//! renderers call [`World::run_schedule`]/[`World::try_run_schedule`] right
//! after building their tree.
//!
//! [`ParsePlugin`] (pulled in by [`StylePlugin`](crate::prelude::StylePlugin))
//! registers the schedule but deliberately does **not** wire it into the main
//! schedule order. A request/response server or a one-shot render must not
//! repaint every frame: doing so re-scans every parsed tree still resident in
//! the world on every tick, so latency climbs as routes accumulate their cached
//! trees. Only a realtime app driven by the main loop wants per-frame repaint,
//! and it opts in with [`RealtimeParsePlugin`].
use beet_core::prelude::*;

/// Registers the [`PostParseTree`] schedule for on-demand runs.
///
/// Does not add it to the main schedule order — see the module docs. Realtime
/// apps add [`RealtimeParsePlugin`] for per-frame repaint.
#[derive(Default)]
pub struct ParsePlugin;

impl Plugin for ParsePlugin {
	fn build(&self, app: &mut App) {
		// ensure the schedule exists for on-demand `run_schedule` callers, without
		// adding it to the per-frame main loop (which would repaint the whole world
		// every tick).
		app.init_schedule(PostParseTree);
	}
}

/// Opt-in plugin for realtime apps driven by the main loop: runs the
/// [`PostParseTree`] pipeline after every [`Update`] so style changes and
/// document mutations repaint each frame.
///
/// A server or one-shot render must not add this — they run [`PostParseTree`]
/// on demand per request/render instead. See the module docs.
#[derive(Default)]
pub struct RealtimeParsePlugin;

impl Plugin for RealtimeParsePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ParsePlugin>()
			.insert_schedule_after(Update, PostParseTree);
	}
}

/// Schedule run once an entity tree has been parsed (or, with
/// [`RealtimeParsePlugin`], after every [`Update`]).
///
/// Hosts style resolution, syntax highlighting, charcell decorations, and the
/// layout/paint pipeline, ordered via their respective system sets.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostParseTree;


#[cfg(test)]
mod test {
	use super::*;
	use bevy::app::MainScheduleOrder;
	use bevy::ecs::schedule::ScheduleLabel;

	fn in_main_order(app: &App) -> bool {
		let label = PostParseTree.intern();
		app.world()
			.resource::<MainScheduleOrder>()
			.labels
			.contains(&label)
	}

	/// The server/one-shot path must not repaint every frame: a request handler
	/// runs [`PostParseTree`] on demand, and leaving it in the main loop re-scans
	/// every parsed route tree each tick, so latency climbs as routes accumulate.
	#[beet_core::test]
	fn parse_plugin_stays_out_of_main_loop() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ParsePlugin));
		in_main_order(&app).xpect_false();
	}

	/// Realtime apps opt into per-frame repaint.
	#[beet_core::test]
	fn realtime_plugin_runs_after_update() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, RealtimeParsePlugin));
		in_main_order(&app).xpect_true();
	}
}
