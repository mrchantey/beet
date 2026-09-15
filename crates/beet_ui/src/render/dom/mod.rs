//! The DOM render target: the browser's document as a third sink over the
//! same [`NodeWalker`] the string and charcell sinks read, driven by the same
//! world.
//!
//! [`DomRenderer`] is the visitor that paints: each visit creates the node an
//! entity paints as and binds the entity to it through [`DomNode`], so a
//! later change patches that node in place rather than repainting, and an
//! event's target resolves to its entity by walking up the DOM, never through
//! a map. [`DomRenderPlugin`] runs the incremental pass every frame after the
//! document sync: a changed [`Value`] patches its text node or its control's
//! property, an attribute entity sets or removes its attribute, and a parent
//! whose children changed reconciles its child list, moving what survived and
//! painting only what is new. Entity identity is the key throughout; there is
//! no positional index.
//!
//! [`DomInputPlugin`] is the other direction: the document's events, resolved
//! to their entities through the same binding and delivered as the events the
//! terminal bridge emits, so the widgets never know which surface they are on.
//!
//! [`NodeWalker`]: crate::prelude::NodeWalker
//! [`Value`]: beet_core::prelude::Value
mod dom_input;
mod dom_node;
mod dom_renderer;
mod dom_sync;
pub use dom_input::*;
pub use dom_node::*;
pub use dom_renderer::*;
pub use dom_sync::*;
/// Harness for the DOM sink tests.
#[cfg(test)]
mod test_ext;
