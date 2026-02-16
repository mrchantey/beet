//! Interactive form control components.
//!
//! Provides components for user input and interaction, semantically
//! equivalent to HTML form elements. These are interface-agnostic —
//! renderers decide how to present them (eg a TUI checkbox, a DOM
//! `<input>`, or a voice prompt).
//!
//! # Components
//!
//! - [`Button`] - clickable action trigger
//! - [`TaskListCheck`] - checkbox marker for list items
use super::node::Node;
use beet_core::prelude::*;


/// A clickable action trigger, semantically equivalent to HTML `<button>`.
///
/// The button label is stored in [`TextNode`](super::TextNode) children,
/// following the standard parent-child text pattern.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[require(Node = Node::new::<Button>())]
pub struct Button;


/// A checkbox marker on a [`ListItem`](super::ListItem), indicating
/// a task list item.
///
/// In markdown this is `- [ ]` (unchecked) or `- [x]` (checked).
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[require(Node = Node::new::<TaskListCheck>())]
pub struct TaskListCheck {
	/// Whether the checkbox is checked.
	pub checked: bool,
}

impl TaskListCheck {
	/// Create an unchecked task list marker.
	pub fn unchecked() -> Self { Self { checked: false } }
	/// Create a checked task list marker.
	pub fn checked() -> Self { Self { checked: true } }
}
