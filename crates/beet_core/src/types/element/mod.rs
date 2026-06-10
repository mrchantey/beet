//! The basic markup-node data types: [`Element`], [`Comment`], [`Doctype`].
//! Authored by every front-end and read by the renderers; pure data, no
//! rendering. An element's attributes are the
//! [`Attribute`](crate::prelude::Attribute) nodes in the snippet module.
mod element;
pub use element::*;
