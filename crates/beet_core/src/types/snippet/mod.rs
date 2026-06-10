//! The `rsx!` / `#[template]` lowering-target runtime, authored around the
//! **snippet** noun.
//!
//! In XML a *node* is a single element or text node; the renderer's individual
//! [`Element`](crate::prelude::Element)/[`Value`](crate::prelude::Value)
//! entities already *are* those nodes, so there is no node struct to add. A
//! **snippet** is a *tree* of nodes: the authored unit that `rsx! { .. }` or a
//! `.bsx` file produces. The snippet lowers to a **template** (the build
//! recipe): template = recipe, snippet = authored source tree.
//!
//! `rsx!` lowers markup to a [`Bundle`](crate::prelude::Bundle) tree built on
//! [`Element`](crate::prelude::Element)/[`Attribute`](crate::prelude::Attribute)/`children!`/[`Value`](crate::prelude::Value),
//! wrapped at the root by [`snippet`] into the
//! [`impl Template<Output = ()>`](bevy::ecs::template::Template) the substrate's
//! `spawn_template` accepts, targeting the beet template substrate directly.
//!
//! - [`IntoSnippet`] lifts markup values (text, `{expr}`, `Vec`, `Option`,
//!   tuple).
//! - [`IntoSnippetBundle`] dispatches an uppercase tag or bare spread: a
//!   [`Component`](crate::prelude::Component) inserts, a build-subtree template
//!   builds.
//! - [`SystemTemplate`] backs `#[template(system)]`.
//! - [`ErrorTemplate`]/[`MissingProps`] carry a graceful build failure.
mod attribute;
mod error_template;
mod snippet;
mod system_template;

pub use attribute::*;
pub use error_template::*;
pub use snippet::*;
pub use system_template::*;
