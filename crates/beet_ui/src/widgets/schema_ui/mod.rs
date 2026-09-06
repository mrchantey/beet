//! The widgets a [`ValueSchema`](beet_core::prelude::ValueSchema) generates
//! rather than an author writing: [`DynamicForm`] (one control per editable
//! leaf), [`DynamicView`] (the read side of the same walk) and [`SchemaEditor`]
//! (a form over the meta-schema, editing the schema itself).
//!
//! The generated controls are the [`controls`](super::controls) ones; what is
//! owned here is the walk that chooses them, the collection/variant controls a
//! schema alone cannot express, and the two rebuild engines
//! ([`SchemaRebuild`] and [`ValueRebuild`]) that regenerate a subtree when the
//! schema or the value shape it was generated from changes.
pub(in crate::widgets) mod collection_edit;
pub(in crate::widgets) mod editor;
mod form;
pub(in crate::widgets) mod schema_rebuild;
pub(in crate::widgets) mod value_rebuild;
pub(in crate::widgets) mod variant_select;
mod view;

pub use editor::*;
pub use form::*;
pub use schema_rebuild::*;
pub use value_rebuild::*;
pub use view::*;
