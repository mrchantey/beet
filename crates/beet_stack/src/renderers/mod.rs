//! Rendering infrastructure for card content trees.
//!
//! This module provides the [`CardWalker`] system parameter and
//! [`CardVisitor`] trait for traversing card entity trees in a
//! consistent, renderer-agnostic way. Concrete renderers implement
//! [`CardVisitor`] to produce output in their target format.
//!
//! # Architecture
//!
//! Rather than each renderer independently querying and walking
//! the entity tree, all renderers share [`CardWalker`] for
//! depth-first traversal with automatic [`Card`](crate::prelude::Card)
//! boundary detection. This ensures consistent traversal semantics
//! across rendering backends.
//!
//! # Available Renderers
//!
//! - Markdown — via [`RenderMarkdown`] (requires `interface` feature)
//! - TUI — via the `draw_system` (requires `tui` feature)
mod card_walker;
pub use card_walker::*;
