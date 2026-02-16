//! Interface-agnostic node representation for beet applications.
//!
//! Beet Stack is an interface-agnostic framework. Rather than
//! prescribing `<div><div><div>` "structure as style" or CSS, the
//! user controls the interface and its styling. Content is described
//! by its *meaning* — a subset of semantic HTML plus markdown types
//! plus form controls — and renderers decide how to present it.
//!
//! # Module Structure
//!
//! - [`node`] - Core [`Node`] component with type invariance enforcement
//! - [`text`] - [`TextNode`], [`Heading`], [`Paragraph`], and semantic markers
//! - [`elements`] - Block and inline content elements (lists, tables, images, etc.)
//! - [`form`] - Interactive form controls (buttons, checkboxes)
//! - [`layout`] - Display and layout primitives ([`DisplayBlock`], [`TextAlignment`])
//! - [`content_macro`] - The [`content!`] macro for ergonomic content composition
//!
//! # Node Invariance
//!
//! Every node type requires a [`Node`] component that records its
//! concrete [`TypeId`](std::any::TypeId). Nodes are invariant — they
//! must not change type after creation. If a different node type is
//! needed, the entity must be despawned and a new one spawned.
//!
//! # Quick Start
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! // Static text with semantic markers
//! let greeting = content![
//!     "Hello, ",
//!     (Important, "world"),
//!     "!"
//! ];
//!
//! // Dynamic text bound to a document field
//! let counter = content![
//!     "Count: ",
//!     FieldRef::new("count").init_with(Value::I64(0))
//! ];
//! ```
//!
//! [`content!`]: crate::content
pub mod content_macro;
mod elements;
mod form;
mod layout;
pub(crate) mod node;
mod text;
pub use content_macro::*;
pub use elements::*;
pub use form::*;
pub use layout::*;
pub use node::*;
pub use text::*;
