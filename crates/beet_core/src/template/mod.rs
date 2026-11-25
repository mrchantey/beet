//! The template spawn substrate: the single path for instantiating entity
//! hierarchies from a Bevy [`Template`](bevy::ecs::template::Template).
//!
//! [`spawn_template`](WorldTemplateExt::spawn_template) and
//! [`insert_template`](EntityWorldMutTemplateExt::insert_template) are the only
//! instantiation entrypoints. Each drives a synchronous build walker that
//! builds the template into its entity, resolves positional slot markers
//! ([`SlotTarget`]/[`SlotChild`]), then fires the lifecycle events
//! ([`SpawnTemplate`] then [`Ready`]). A failed root rides
//! [`TemplateError`] rather than panicking.
//!
//! `app.install_types::<T>()` installs the [`ReflectTemplate`] bridge so a
//! template resolves by name.
//!
//! This is the no_std core; async helpers sit behind the `bevy_async` feature,
//! never on the core path.

#[cfg(feature = "bevy_asset")]
mod asset;
mod entity_map;
// templates held unbuilt, built on demand by their holder
mod held;
mod instance;
mod lifecycle;
mod prefab;
mod registry;
#[cfg(feature = "serde")]
mod scene_body_builder;
mod slot;
mod spawn_template;
mod template_plugin;

#[cfg(feature = "bevy_asset")]
pub use asset::*;
pub use entity_map::*;
pub use held::*;
pub use instance::*;
pub use lifecycle::*;
pub use prefab::*;
pub use registry::*;
#[cfg(feature = "serde")]
pub use scene_body_builder::*;
pub use slot::*;
pub use spawn_template::*;
pub use template_plugin::*;
