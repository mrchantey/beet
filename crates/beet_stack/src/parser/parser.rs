//! The core [`Parser`] trait for diffing content against an entity tree.
//!
//! Parsers take some input format and reconcile it with an existing
//! Bevy entity hierarchy, spawning, updating, or despawning entities
//! as needed to match the parsed content.
use beet_core::prelude::*;

/// Trait for reconciling parsed content against an entity hierarchy.
///
/// A parser takes ownership of some input (eg a markdown string) and
/// diffs it against the children of the provided entity, making
/// minimal mutations to bring the entity tree in line with the parsed
/// content.
///
/// # Example
///
/// ```ignore
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = World::new();
/// let root = world.spawn_empty().id();
/// MarkdownDiffer::new("# Hello\n\nworld")
///     .diff(&mut world.entity_mut(root));
/// ```
pub trait Parser {
	/// Reconcile the parsed content with the entity's children.
	///
	/// Takes the [`EntityWorldMut`] by value because implementations
	/// typically need world access beyond the single entity.
	/// Implementations should diff positionally against existing
	/// children: matching node types are updated in place, mismatches
	/// cause despawn and re-spawn, extra old children are removed,
	/// and extra new nodes are appended.
	fn diff(&mut self, entity: EntityWorldMut) -> Result;
}
