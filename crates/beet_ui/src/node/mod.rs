//! The `rsx!` / `#[template]` lowering-target runtime.
//!
//! `rsx!` lowers markup to a [`Bundle`](beet_core::prelude::Bundle) tree built
//! on [`Element`](crate::prelude::Element)/[`Attribute`](crate::prelude::Attribute)/`children!`/[`Value`](beet_core::prelude::Value),
//! wrapped at the root by [`node`] into the
//! [`impl Template<Output = ()>`](bevy::ecs::template::Template) the substrate's
//! `spawn_template` accepts. This is the no-`bevy_scene` authoring layer: it
//! targets the beet template substrate directly.
//!
//! - [`IntoNode`] lifts markup values (text, `{expr}`, `Vec`, `Option`, tuple).
//! - [`IntoNodeBundle`] dispatches an uppercase tag or bare spread: a
//!   [`Component`](beet_core::prelude::Component) inserts, a build-subtree
//!   template builds.
//! - [`SystemTemplate`] backs `#[template(system)]`.
//! - [`ErrorTemplate`]/[`MissingProps`] carry a graceful build failure.
mod into_node;
mod node_error;
mod node_ext;
mod system_node;

pub use into_node::*;
pub use node_error::*;
pub use node_ext::*;
pub use system_node::*;
