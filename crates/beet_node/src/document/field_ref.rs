use crate::prelude::*;
use beet_core::prelude::*;



/// A reference to a specific field in a document.
///
/// Used by content and actions to interact with document fields. By default,
/// fields are initialized with `null` if missing, unless configured otherwise
/// via [`on_missing`](FieldRef::on_missing).
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
	SetWith,
)]
#[reflect(Component)]
#[component(immutable, on_add=on_add)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldRef {
	/// The path to the document
	pub document: DocumentPath,
	/// The path to the field within the document
	pub field_path: FieldPath,
	/// Behavior when the field is missing from the document.
	pub on_missing: OnMissingField,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	if !world.entity(cx.entity).contains::<Value>() {
		let this = world.entity(cx.entity).get::<FieldRef>().unwrap();
		let value = match this.on_missing.clone() {
			OnMissingField::Init { value } => value,
			_ => Value::default(),
		};
		world.commands().entity(cx.entity).insert(value);
	}
}


impl FieldRef {
	/// Create a new field reference with the given field path.
	///
	/// Uses the default [`DocumentPath::Ancestor`] for document resolution.
	/// Use [`with_document`](Self::with_document) to specify a different document.
	///
	/// By default, missing fields are initialized with [`Value::Null`].
	pub fn new<M>(field_path: impl IntoFieldPath<M>) -> Self {
		Self {
			document: DocumentPath::default(),
			field_path: field_path.into_field_path(),
			on_missing: OnMissingField::default(),
		}
	}

	pub fn of<T: TypePath>() -> Self { Self::new(FieldPath::of::<T>()) }

	/// Set this field reference to error if the field is missing instead of initializing it.
	pub fn error_on_missing(mut self) -> Self {
		self.on_missing = OnMissingField::EmitError;
		self
	}

	/// Set the behavior when the field is missing.
	pub fn on_missing(mut self, on_missing: OnMissingField) -> Self {
		self.on_missing = on_missing;
		self
	}

	/// Set the field to initialize with a specific value if missing.
	pub fn with_init(mut self, value: impl Into<Value>) -> Self {
		self.on_missing = OnMissingField::Init {
			value: value.into(),
		};
		self
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use std::ops::Deref;

	#[test]
	fn field_ref_new() {
		let field = FieldRef::new("field");

		field.document.xpect_eq(DocumentPath::Ancestor);
		field
			.field_path
			.deref()
			.xpect_eq(vec![FieldSegment::key("field")]);
		field
			.on_missing
			.xpect_eq(OnMissingField::Init { value: Value::Null });
	}
}
