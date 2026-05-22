//! Scheduling for the post-parse pipeline.
//!
//! [`PostParseTree`] is the single schedule where a parsed entity tree is
//! styled, decorated, laid out, and painted. Parsers run it as a one-off after
//! building their tree; [`ParsePlugin`] additionally wires it into the main
//! schedule order so realtime apps re-run it after every [`Update`].
use beet_core::prelude::*;

/// Registers [`PostParseTree`] to run after [`Update`] in the main schedule
/// order, so realtime apps resolve styles and repaint each frame.
///
/// One-off parsers (eg [`MarkdownParser`](crate::prelude::MarkdownParser)) run
/// the schedule directly via [`World::try_run_schedule`] after building their
/// tree, so they do not require this plugin — it only matters for apps driven
/// by the main loop.
#[derive(Default)]
pub struct ParsePlugin;

impl Plugin for ParsePlugin {
	fn build(&self, app: &mut App) {
		// for realtime apps, run the post-parse pipeline after every update
		app.insert_schedule_after(Update, PostParseTree);
	}
}

/// Schedule run once an entity tree has been parsed (or, in realtime apps,
/// after every [`Update`]).
///
/// Hosts style resolution, syntax highlighting, charcell decorations, and the
/// layout/paint pipeline, ordered via their respective system sets.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostParseTree;
