//! A scene document described at the schema level.
//!
//! The scene is the `template_serde` shape (resources plus file-keyed entities,
//! each a map of component values keyed by type path), and it is an ordinary
//! registry entry rather than a widget's special case: each component value is
//! described by its key ([`MapSchema::Keyed`](crate::prelude::MapSchema)), so
//! the keystone closure holds for scenes exactly as it does for the
//! meta-schema. What the
//! schema cannot say, a relation's acyclicity, is registered type data
//! ([`RelationMeta`]) checked over the whole document ([`SceneEntities`]).

mod relation_meta;
mod scene_entities;
mod scene_schema;
pub use relation_meta::*;
pub use scene_entities::*;
