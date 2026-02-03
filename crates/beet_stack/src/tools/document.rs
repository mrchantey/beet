use beet_core::prelude::*;


/// A tool that increments a specified field when triggered, returning the new value.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Increment {
	/// Path to the field to increment.
	pub field: FieldRef,
}

/// A reference to a specific field For tools that interact with documents,
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// By default fields are initialized with a default value
	/// if missing, otherwise an error is triggered.
	pub error_on_missing: bool,
}




/// Specify the document to operate on, either the global application-level document,
/// or another by id.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
pub enum DocumentPath {
	/// The global application-level document
	#[default]
	App,
	/// A specific document by entity id
	Entity(Entity),
}
