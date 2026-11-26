//! [`PostParseTree`], the schedule a parsed tree finishes in.
use bevy::ecs::schedule::ScheduleLabel;

/// The schedule run once a document has been parsed into a world (or, with
/// `beet_ui`'s `RealtimeParsePlugin`, after every [`Update`]).
///
/// Hosts style resolution, syntax highlighting, charcell decorations and the
/// layout and paint pipeline, ordered by their system sets.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostParseTree;
