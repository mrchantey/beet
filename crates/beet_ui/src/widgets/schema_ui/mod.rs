//! The widgets a [`ValueSchema`](beet_core::prelude::ValueSchema) generates
//! rather than an author writing: [`DynamicForm`] (one control per editable
//! leaf), [`DynamicView`] (the read side of the same walk) and [`SchemaEditor`]
//! (a form over the meta-schema, editing the schema itself).
//!
//! The generated controls are the [`controls`](super::controls) ones; what is
//! owned here is the walk that chooses them ([`form`], with an arm file per
//! responsibility), the collection/variant controls a schema alone cannot
//! express, and the two rebuild engines ([`schema_rebuild`] and
//! [`value_rebuild`]) that regenerate a subtree when the schema or the value
//! shape it was generated from changes.
//!
//! Only the widgets themselves and [`UneditableField`] leave the widget set: the
//! rebuild engines, the generated controls' marks and the dispatch arms are the
//! machinery behind them, not an authoring surface.
pub(in crate::widgets) mod collection_edit;
pub(in crate::widgets) mod component_picker;
mod composite_field;
mod dependent_field;
pub(in crate::widgets) mod editor;
pub(in crate::widgets) mod entity_picker;
pub(in crate::widgets) mod field_layout;
mod form;
mod scalar_field;
pub(in crate::widgets) mod schema_rebuild;
/// Harness for the schema-driven widget tests.
#[cfg(test)]
pub(in crate::widgets) mod test_ext;
pub(in crate::widgets) mod value_rebuild;
pub(in crate::widgets) mod variant_select;
mod view;

pub use editor::*;
pub use form::*;
pub use view::*;

use beet_core::prelude::*;
use bevy::reflect::TypeRegistry;

/// The resolver a schema-driven widget generates against: the by-name
/// registry and bevy's type registry, whichever the world holds.
///
/// Both are optional because a widget may build before [`DocumentPlugin`]
/// has initialized the one or in a world without the other; every
/// indirection then defers exactly as a schema still arriving does.
pub(in crate::widgets) fn resolver<'a>(
	schemas: Option<&'a SchemaRegistry>,
	types: Option<&'a TypeRegistry>,
) -> SchemaResolver<'a> {
	let mut resolver = SchemaResolver::default();
	if let Some(schemas) = schemas {
		resolver = resolver.with_schemas(schemas);
	}
	if let Some(types) = types {
		resolver = resolver.with_types(types);
	}
	resolver
}
