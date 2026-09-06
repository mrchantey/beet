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
mod collection_edit;
mod composite_field;
mod dependent_field;
pub(in crate::widgets) mod editor;
mod field_layout;
mod form;
mod scalar_field;
pub(in crate::widgets) mod schema_rebuild;
/// Harness for the schema-driven widget tests.
#[cfg(test)]
mod test_ext;
pub(in crate::widgets) mod value_rebuild;
pub(in crate::widgets) mod variant_select;
mod view;

pub use editor::*;
pub use form::*;
pub use view::*;
