//! Interface-agnostic content representation for beet applications.
//!
//! This module provides primitives for describing content by its *meaning*
//! rather than its visual appearance. This semantic approach enables content
//! to be appropriately presented across diverse interfaces - from visual UIs
//! to voice assistants to accessibility tools.
//!
//! # Key Concepts
//!
//! ## Semantic over Styling
//!
//! Instead of specifying "bold" or "italic", we use semantic markers like
//! [`Important`] and [`Emphasize`]. Each interface can then render these
//! appropriately:
//!
//! - A visual UI renders [`Important`] as bold text
//! - A voice assistant might use a louder or more emphatic tone
//! - A Braille display can indicate importance through its conventions
//!
//! ## Static and Dynamic Content
//!
//! Content can be either static ([`TextNode`] with a string) or dynamic
//! (bound to a [`FieldRef`] that syncs with a [`Document`]).
//!
//! ## Composable Structure
//!
//! Use the [`content!`] macro to compose content segments with mixed static content,
//! semantic markers, and dynamic field bindings.
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
//! [`Document`]: crate::document::Document
//! [`FieldRef`]: crate::document::FieldRef
//! [`content!`]: crate::content
pub mod content_macro;
mod elements;
mod layout;
mod text;
mod text_query;
pub use content_macro::*;
pub use elements::*;
pub use layout::*;
pub use text::*;
pub use text_query::*;
