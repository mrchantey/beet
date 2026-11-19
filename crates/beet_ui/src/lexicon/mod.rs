//! Lexicon objects as trees: a tree rendered as one atproto lexicon object,
//! and one parsed back onto entities.
//!
//! A lexicon names the schema of a value inside a record by NSID, ie
//! `org.beet.scene#body` or `pub.leaflet.blocks.text`, and the value is
//! written as json or DAG-CBOR alike. A media type names a whole document's
//! bytes, negotiated per request ([`DocRenderers`], [`DocParsers`]), which is
//! a different axis, so lexicons get their own pair of registries, keyed by
//! NSID and working on [`OpenUnion`] values rather than bytes:
//!
//! - [`LexiconRenderers`]: a [`LexiconRenderer`] per def a caller asks for,
//!   rendering the tree at an entity as a member of that def. A renderer walks
//!   the tree itself, so the members nested in its output are its own.
//! - [`LexiconParsers`]: a [`LexiconParser`] per def, every def a reader
//!   understands registered on its own (`pub.leaflet.blocks.text`, not one
//!   `pub.leaflet.content`), so each member nested in another parses back
//!   through the registry onto a child entity
//!   ([`LexiconParseContext::parse_child`]) and a reader degrades member by
//!   member.
//!
//! Both are installed by value, `app.install(SceneBodyLexicon)`, the last
//! registered for a def winning, so a downstream crate replaces a built-in by
//! registering its own. Rendering a def nothing registered is an error naming
//! the registered ones.
//!
//! # Unknown members
//!
//! Parsing never fails on a def it does not know: the member lands whole on
//! its entity as an [`UnknownLexicon`], and any renderer reaching that entity
//! writes it back unchanged, so a round trip through a reader that knows half
//! a document loses nothing.
//!
//! # Encoding
//!
//! A renderer answers plain values and a parser reads them: the record a
//! member travels in is an `AtprotoValue`, which encodes floats and bytes into
//! the data model on the way out and decodes them on the way in, so neither
//! half sees `org.beet.core#float`. A member referencing a blob resolves it
//! against the [`LexiconSource`] its record was read from.
//!
//! # The targets
//!
//! - `org.beet.scene#body` ([`SceneBodyLexicon`], with `template_serde`): the
//!   tree as a scene body, every component keyed by type path and read back
//!   without loss. A standard site document's `content` by default.
//! - Leaflet (`pub.leaflet.content` and its blocks, `beet_router`'s
//!   `standard_site_extra` feature): the tree as Leaflet's block model, lossy
//!   by design, and a Leaflet document read into the elements the markdown
//!   parser produces.
//!
//! [`OpenUnion`]: beet_core::prelude::OpenUnion
mod lexicon_parsers;
mod lexicon_renderers;
#[cfg(feature = "template_serde")]
mod scene_body;
mod unknown_lexicon;
pub use lexicon_parsers::*;
pub use lexicon_renderers::*;
#[cfg(feature = "template_serde")]
pub use scene_body::*;
pub use unknown_lexicon::*;
