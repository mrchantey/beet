//! Text traversal system parameter.
//!
//! Provides [`TextQuery`] for collecting text from structural elements
//! ([`Heading`], [`Paragraph`]) by walking their [`TextContent`] children.
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
	display_blocks: Query<'w, 's, (), With<DisplayBlock>>,
	headings: Query<'w, 's, &'static Heading>,
}

impl TextQuery<'_, '_> {
	/// Returns the plain text of a structural element by concatenating
	/// all [`TextContent`] children in order, ignoring inline markers.
	/// Skips nested structural elements.
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

	/// Returns the entity and [`TextContent`] pairs that are direct
	/// children of a structural element, skipping nested structural
	/// elements.
	pub fn collect_text_entities(
		&self,
		parent: Entity,
	) -> Vec<(Entity, &TextContent)> {
		let mut result = Vec::new();
		if let Ok(children) = self.children.get(parent) {
			for child in children.iter() {
				if self.is_structural(child) {
					continue;
				} else if let Ok(text) = self.text.get(child) {
					result.push((child, text));
				}
			}
		}
		result
	}

	/// Returns true if the entity is a [`DisplayBlock`] element,
	/// ie a [`Heading`] or [`Paragraph`].
	pub fn is_structural(&self, entity: Entity) -> bool {
		self.display_blocks.contains(entity)
	}

	/// Returns true if the entity has a [`Heading`] component.
	pub fn is_heading(&self, entity: Entity) -> bool {
		self.headings.contains(entity)
	}

	/// Returns the [`Heading`] level for an entity, if it has one.
	pub fn heading_level(&self, entity: Entity) -> Option<u8> {
		self.headings
			.get(entity)
			.ok()
			.map(|heading| heading.level())
	}

	/// Finds the first [`Heading1`] entity among the direct children
	/// of `parent` and returns its collected text.
	///
	/// Useful for extracting a card's main title without walking
	/// the entire subtree.
	pub fn main_heading(&self, parent: Entity) -> Option<(Entity, String)> {
		let children = self.children.get(parent).ok()?;
		for child in children.iter() {
			if self
				.headings
				.get(child)
				.is_ok_and(|heading| heading.level() == 1)
			{
				let text = self.collect_text(child);
				return Some((child, text));
			}
		}
		None
	}
}
