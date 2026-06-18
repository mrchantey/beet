//! The basic markup-node data types: [`Element`], [`Comment`], [`Doctype`].
//! Authored by every front-end and read by the renderers; pure data, no
//! rendering. An element's attributes are the
//! [`Attribute`](crate::prelude::Attribute) nodes in the snippet module;
//! [`ElementTraverseQuery`] is the single ancestry walk that bridges the
//! `ChildOf` and `AttributeOf` trees, and [`ElementTextQuery`] is its descendant
//! counterpart for an element's text content.
mod element;
mod element_text;
mod element_traverse;
pub use element::*;
pub use element_text::*;
pub use element_traverse::*;
