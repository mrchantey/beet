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
//! - [`node`] - Core [`Node`] component with [`NodeKind`] dispatch enum
//! - [`text`] - [`TextNode`], [`Heading`], [`Paragraph`], and semantic markers
//! - [`elements`] - Block and inline content elements (lists, tables, images, etc.)
//! - [`form`] - Interactive form controls (buttons, checkboxes)
//! - [`layout`] - Display and layout primitives ([`DisplayBlock`], [`TextAlignment`])
//! - [`style`] - [`InlineModifier`] bitflags, [`InlineStyle`], and [`VisitContext`]
//! - [`content_macro`] - The [`content!`] macro for simple bundle composition
//!
//! # Node Invariance
//!
//! Every node type requires a [`Node`] component that records its
//! concrete [`NodeKind`]. Nodes are invariant — they must not change
//! type after creation. If a different node type is needed, the
//! entity must be despawned and a new one spawned.
//!
//! # Inline Container Pattern
//!
//! Inline marker components ([`Important`], [`Emphasize`], [`Code`],
//! [`Link`], etc.) and [`TextNode`] are mutually exclusive on the
//! same entity, following the HTML model. Inline markers are
//! containers whose children include [`TextNode`] entities:
//!
//! ```
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! // Correct: Important is a container with TextNode child
//! let bold = (Important, children![TextNode::new("bold")]);
//!
//! // For ergonomic markdown-based content, use the markdown! macro:
//! // let root = markdown!(world, "Hello **world**!");
//! ```
pub mod content_macro;
mod elements;
mod form;
mod layout;
pub(crate) mod node;
mod style;
mod text;
pub use content_macro::*;
pub use elements::*;
pub use form::*;
pub use layout::*;
pub use node::*;
pub use style::*;
pub use text::*;
