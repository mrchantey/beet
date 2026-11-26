//! Template serialization and deserialization.
//!
//! Every serde format reads as one [`SceneBody`], which reflects into a
//! [`DynamicTemplate`] that builds itself into the world through the
//! [`spawn_template`](WorldTemplateExt::spawn_template) path. A
//! fully resolved save-game and a hand-authored page are the same kind of thing
//! at different points on one axis: how much of the template is a resolved value
//! versus a deferred build. This collapses component-only world serialization and
//! hierarchy-producing construction into "build a template".
//!
//! The module is no_std at its core: the representation, walker, and value-slot
//! serde never reference an asset type. Assets are an additive feature.
//!
//! The serialized document is `{ resources, entities }`, flat at every level:
//! the resources are a map of type path to value and so is each entity, keyed
//! by file key, a reference to an entity being its key (see [`SceneBody`]'s
//! module for the wire shape). The body is plain data and lives with the
//! other value types; this module is its way to and from a world.
//!
//! # `org.beet.scene#body`
//!
//! The wire shape is the scene body, [`SceneBody`], and as an atproto lexicon
//! object it is `org.beet.scene#body`: the same `{resources, entities}` with
//! `"$type": "org.beet.scene#body"` at its root, which is how any structured
//! document says it is a scene body. A standard site document carries one
//! inline as its `content`, the post itself, rendered and parsed by
//! `beet_ui`'s `SceneBodyLexicon`. Its components are keyed by Rust type path,
//! not NSID, and published as the saver writes them, a reader keeping the ones
//! it understands; floats and bytes take the data model's forms at the record
//! boundary (`AtprotoValue`), never in the body.
//!
//! It is not the `org.beet.scene` record, which describes one compiled
//! document, its provenance, its scene meta and the `BlobRef` of its body, a
//! postcard blob beside it. The body is the scene; the record is about one.
//!
//! # Unregistered components
//!
//! A body is read as values and needs no type registry; reflection happens
//! when it spawns. A component whose type path this binary never registered is
//! left off its entity and its value held in the entity's
//! [`Unregistered`], which is also how markup's unknown tag or
//! spread arrives, so the entity and its children survive, `beet check`, which
//! registers everything, elevates it to an error, and a strict caller refuses
//! through [`UnregisteredQuery`]. A saver writes each held component back as
//! one of the entity's own keys, so a round trip through a binary lacking a
//! plugin loses nothing, and a binary with it reads the component on the next
//! load.
//!
//! # High-level API
//!
//! - [`TemplateSaver`] - Serialize a world or entity subtree to a structured
//!   format, as a [`SceneBody`].
//! - [`TemplateLoader`] - Deserialize bytes into a [`DynamicTemplate`] and build
//!   it through `spawn_template`.
//!
//! # Building blocks
//!
//! - [`SceneBody`] - The scene as data, the serde form of every format: its
//!   resources and its entities, each a map of type path to [`Value`], and
//!   [`DynamicTemplate::reflect`], the unregistered rule.
//! - [`DynamicTemplate`] - A body reflected against the type registry: ordered
//!   resources and entities, each entity's component slots a resolved value or
//!   a deferred template.
//! - [`TemplateBuilder`] - Extracts a [`SceneBody`] from a
//!   [`World`].
//! - [`TypeFilter`] - the rules for which types are extracted.
//! - [`TemplateEntityMap`] - The file-key to world-entity map a loaded document
//!   retains, so a re-save writes every entity back under the key it loaded from.
//! - [`SceneDocument`] - A loaded scene as an editable [`Document`] the world
//!   follows per component, forked from an authored original on first boot.

#[cfg(doc)]
use beet_core::prelude::*;

mod body_components;
mod cfg_prune;
#[cfg(all(test, feature = "json"))]
mod conformance;
mod dynamic_template;
mod loader;
mod saver;
mod scene_document;
mod scene_reflect;
mod scene_source;
mod template_builder;

pub use body_components::*;
pub(self) use cfg_prune::*;
pub use dynamic_template::*;
pub use loader::LoadTemplateSerde;
pub(self) use loader::Placement;
pub use loader::TemplateLoader;
pub use saver::*;
pub use scene_document::SceneDocument;
pub use scene_document::SceneFork;
pub(crate) use scene_document::sync_scene_documents;
#[cfg(feature = "bsx")]
pub(crate) use scene_reflect::EntityKeys;
pub(crate) use scene_reflect::KeySentinels;
pub use scene_source::*;
pub use template_builder::*;
