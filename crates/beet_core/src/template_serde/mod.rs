//! Template serialization and deserialization.
//!
//! Every serde format deserializes into one [`DynamicTemplate`], which builds
//! itself into the world through the
//! [`spawn_template`](crate::prelude::WorldTemplateExt::spawn_template) path. A
//! fully resolved save-game and a hand-authored page are the same kind of thing
//! at different points on one axis: how much of the template is a resolved value
//! versus a deferred build. This collapses component-only world serialization and
//! hierarchy-producing construction into "build a template".
//!
//! The module is no_std at its core: the representation, walker, and value-slot
//! serde never reference an asset type. Assets are an additive feature.
//!
//! # High-level API
//!
//! - [`TemplateSaver`] - Serialize a world or entity subtree to RON, JSON, or
//!   postcard, producing a resolved-value [`DynamicTemplate`].
//! - [`TemplateLoader`] - Deserialize bytes into a [`DynamicTemplate`] and build
//!   it through `spawn_template`.
//!
//! # Building blocks
//!
//! - [`DynamicTemplate`] - The intermediate representation: ordered resources and
//!   nodes, each node's component slots a resolved value or a deferred template.
//! - [`TemplateBuilder`] - Extracts a resolved-value [`DynamicTemplate`] from a
//!   [`World`](bevy::prelude::World).
//! - [`TemplateFilter`] - Allow/deny lists controlling which types are extracted.
//! - [`DynamicTemplateSerializer`] / [`DynamicTemplateDeserializer`] - The serde
//!   implementations for the resolved-value form.

#[cfg(feature = "bevy_asset")]
mod asset;
mod dynamic_template;
mod loader;
mod saver;
mod template_builder;
mod template_filter;
// kept private so the `serde` name does not shadow the `serde` crate when this
// module is glob re-exported into the crate prelude.
mod serde;

#[cfg(feature = "bevy_asset")]
pub use asset::*;
pub use dynamic_template::*;
pub use loader::*;
pub use saver::*;
pub use serde::DynamicTemplateDeserializer;
pub use serde::DynamicTemplateSerializer;
pub use template_builder::*;
pub use template_filter::*;
