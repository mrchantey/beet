//! The basic markup-node data types: [`Element`], [`Attribute`],
//! [`Attributes`], [`AttributeOf`], [`Comment`], [`Doctype`]. Authored by every
//! front-end and read by the renderers; pure data, no rendering.
mod element;
pub use element::*;
