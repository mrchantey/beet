//! The template spawn substrate: the single blessed path for instantiating
//! entity hierarchies from a Bevy [`Template`](bevy::ecs::template::Template).
//!
//! [`spawn_template`](WorldTemplateExt::spawn_template) and
//! [`insert_template`](EntityWorldMutTemplateExt::insert_template) are the only
//! instantiation entrypoints. Each drives a synchronous build walker that
//! builds the template into its entity, resolves positional slot markers
//! ([`SlotTarget`]/[`SlotChild`]), then fires the lifecycle events
//! ([`SpawnTemplate`] then [`LoadTemplate`]). A failed root rides
//! [`TemplateError`] rather than panicking.
//!
//! [`register_template`](WorldRegisterTemplateExt::register_template) installs
//! the [`ReflectTemplate`] bridge so a template resolves by name.
//!
//! This is the no_std core every later task builds against; async helpers sit
//! behind the `bevy_async` feature, never on the core path.

mod lifecycle;
mod registry;
mod slot;
mod spawn_template;
mod template_plugin;

pub use lifecycle::*;
pub use registry::*;
pub use slot::*;
pub use spawn_template::*;
pub use template_plugin::*;
