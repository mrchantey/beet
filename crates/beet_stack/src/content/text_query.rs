//! Text traversal system parameter.
//!
//! Provides [`TextQuery`] for collecting text from structural elements
//! ([`Title`], [`Paragraph`]) by walking their [`TextContent`] children.
//! Handles inline markers ([`Important`], [`Emphasize`], etc.) and
//! respects structural boundaries.
use crate::prelude::*;
use beet_core::prelude::*;

/// System parameter for traversing text content within structural elements.
///
/// Collects [`TextContent`] from children of a structural parent,
/// respecting inline markers and structural boundaries.
#[derive(SystemParam)]
pub struct TextQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	text: Query<'w, 's, &'static TextContent>,
	titles: Query<'w, 's, (), With<Title>>,
	paragraphs: Query<'w, 's, (), With<Paragraph>>,
	ancestors: Query<'w, 's, &'static ChildOf>,
}

impl TextQuery<'_, '_> {
	/// Returns the plain text of a structural element by concatenating
	/// all [`TextContent`] children in order, ignoring inline markers.
	/// skips nested structural elements.
	pub fn collect_text(&self, parent: Entity) -> String {
		let mut result = String::new();
		if let Ok(children) = self.children.get(parent) {
			for child in children.iter() {
				// Skip nested structural elements
				if self.is_structural(child) {
					continue;
				} else if let Ok(text) = self.text.get(child) {
					result.push_str(text.as_str());
				}
			}
		}
		result
	}

	/// Returns the entities that are direct [`TextContent`] children
	/// of a structural element, skipping nested structural elements.
	pub fn collect_text_entities(&self, parent: Entity) -> Vec<Entity> {
		let mut result = Vec::new();
		if let Ok(children) = self.children.get(parent) {
			for child in children.iter() {
				if self.is_structural(child) {
					continue;
				} else if self.text.contains(child) {
					result.push(child);
				}
			}
		}
		result
	}

	/// Returns true if the entity is a structural element
	/// ([`Title`] or [`Paragraph`]).
	pub fn is_structural(&self, entity: Entity) -> bool {
		self.titles.contains(entity) || self.paragraphs.contains(entity)
	}

	/// Returns true if the entity is a [`Title`].
	pub fn is_title(&self, entity: Entity) -> bool {
		self.titles.contains(entity)
	}

	/// Compute the nesting depth of a [`Title`] by walking ancestors.
	///
	/// Level 0 is the root title. Each ancestor [`Title`] increments
	/// the level by one.
	// TODO what about older cousins? structurally thats still
	pub fn title_level(&self, entity: Entity) -> u8 {
		let mut level: u8 = 0;
		let mut current = entity;
		while let Ok(child_of) = self.ancestors.get(current) {
			let parent = child_of.parent();
			if self.titles.contains(parent) {
				level += 1;
			}
			current = parent;
		}
		level
	}
}


/// Observer that calculates [`TitleLevel`] when a [`Title`] is inserted.
///
/// Walks up the ancestor tree, counting sibling [`Title`] components
/// in parent containers. Level 0 is the main/root title.
pub(crate) fn calculate_title_level(
	trigger: On<Insert, Title>,
	ancestors: Query<&ChildOf>,
	titles: Query<(), With<Title>>,
	children_query: Query<&Children>,
	mut commands: Commands,
) {
	let entity = trigger.entity;
	let mut level: u8 = 0;

	// Walk up the ancestor chain
	let mut current = entity;
	while let Ok(child_of) = ancestors.get(current) {
		let parent = child_of.parent();

		// Check if the parent itself is a Title
		if titles.contains(parent) {
			level += 1;
		}

		// Check if any sibling of our ancestor (in the parent's children)
		// is a Title and comes before our ancestor in the child list.
		// Actually, per the plan: "a sibling of the parent may also
		// contain a Title, that should count as a parent title."
		// This means any Title among the parent's children (excluding
		// the current ancestor path) counts as a nesting level.
		if let Ok(siblings) = children_query.get(parent) {
			for sibling in siblings.iter() {
				if sibling == current {
					// We've reached our position, stop counting siblings
					break;
				}
				if titles.contains(sibling) {
					level += 1;
				}
			}
		}

		current = parent;
	}

	commands.entity(entity).insert(TitleLevel(level));
}
