//! Office Open XML: Word files, workbooks and slide decks as the one tree.
//!
//! A file is read into one tree whatever its format, so nothing downstream
//! knows which format it came from. The root carries what the format says
//! about the whole, the [`PageMeta`] its core properties declare and the
//! [`OoxmlPackage`] it came in, and each part read is an
//! [`OoxmlNode::Part`] beneath it, every node of the part an entity carrying
//! the [`OoxmlNode`] it was read as:
//!
//! - **HTML where it maps cleanly.** A node that means what an HTML element
//!   means gets that [`Element`]: a heading style `h1` to `h6`, a paragraph
//!   `p`, a table `table`, `tr` and `td`, a hyperlink `a`, a break `br`, a
//!   picture `img`, a checkbox control `<input type="checkbox">`, a tracked
//!   insertion `ins` and deletion `del`, alternate content's fallback
//!   `template`, a run by its look `mark`, `strong`, `em` or a styled `span`.
//!   HTML attributes carry what HTML says: `colspan`, `href`, `checked`,
//!   `style`.
//! - **Every other node is still an entity, with no [`Element`].** A render
//!   walks through it, so a property element, a bookmark or a content
//!   control's wrapper stays where the file put it at no cost to any
//!   renderer. Text no reader sees, ie a field's instruction, is an
//!   [`OoxmlNode::Text`] rather than a [`Value`].
//! - **What HTML cannot say is components on the same entities**: what a
//!   query wants, a paragraph's [`ParagraphStyle`], a [`ListLevel`], a run's
//!   whole [`RunLook`], a cell's [`TableCellAddress`] or [`SheetCellAddress`].
//! - **An entity with no [`OoxmlNode`] is projection only**, ie a list's
//!   `<ul>` or the `<strong>` inside a highlighted bold run. The writer
//!   passes through it, writing its children in place.
//!
//! # Editing and writing
//!
//! An edit is an ECS edit in HTML terms: a [`Value`] changed, `checked` set,
//! a node removed, a projection-only node added. Writing back is
//! re-projection: [`OoxmlRenderer`] answers a Word file's or a workbook's
//! own media type by writing every [`OoxmlNode`] as it says, and anything an
//! edit made with none by the reverse of the mapping, new content in its
//! neighbour's style. A deck is read, never written. An untouched file writes back render
//! identical, every part, relationship and attribute kept and only lexical
//! form free, and an edited one differs only where it was edited.
//!
//! # Formats
//!
//! | Format | Read as |
//! |---|---|
//! | Word | its headers, main document, footers and notes, the main document's tables numbered as a fill addresses them and captioned `<!-- t<n> -->` |
//! | Slide deck | each listed slide a `<section>` in presentation order headed `Slide <n>`, its shapes in reading order, each text frame labelled by its kind, its tables, its SmartArt read from its data part, its pictures `<img>`s with their [`SlidePicture`], and its speaker notes an `<aside>`; [`OoxmlPlugin`] appends a triage of the deck in [`PostParseTree`] |
//! | Workbook | each worksheet a `<table>` captioned with its tab name, every cell a [`SheetCellAddress`] with its [`CellLocked`] and [`CellFormula`], shared strings resolved, a merged range's first cell spanning the rest, and every gap before a row's last cell a projection-only `<td>` |
//!
//! [`PageMeta`]: crate::prelude::PageMeta
//! [`PostParseTree`]: crate::prelude::PostParseTree
//! [`TableCellAddress`]: crate::prelude::TableCellAddress
//! [`Element`]: beet_core::prelude::Element
//! [`Value`]: beet_core::prelude::Value
mod core_properties;
mod deck;
mod deck_report;
mod namespace;
mod node;
mod ooxml_plugin;
mod ooxml_query;
mod package;
mod parser;
mod renderer;
mod sheet_cell;
mod word;
mod workbook;
mod writer;
pub use core_properties::*;
pub use deck::*;
pub use deck_report::DeckReported;
pub use namespace::*;
pub use node::*;
pub use ooxml_plugin::*;
pub use package::*;
pub use parser::*;
pub use renderer::*;
pub use sheet_cell::*;
pub use word::*;
