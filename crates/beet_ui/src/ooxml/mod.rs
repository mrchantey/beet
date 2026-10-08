//! Office Open XML: Word files, workbooks and slide decks as the one tree.
//!
//! A file is read into one tree whatever its format, so nothing downstream
//! knows which format it came from. The root carries what the format says
//! about the whole, the [`PageMeta`] its core properties declare and the
//! [`OoxmlPackage`] it came in, and each part read is a [`SourcePart`]
//! beneath it, every node of the part an entity:
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
//!   renderer. Text no reader sees, ie a field's instruction, is a
//!   [`SourceText`] rather than a [`Value`].
//! - **What HTML cannot say is components on the same entities**: every
//!   source node's [`SourceElement`], and what a query wants, a paragraph's
//!   [`ParagraphStyle`], a [`ListLevel`], a run's whole [`RunLook`], a cell's
//!   [`TableCellAddress`].
//! - **An entity with no source identity is projection only**, ie a list's
//!   `<ul>` or the `<strong>` inside a highlighted bold run. A writer passes
//!   through it and writes its children in place.
//!
//! # Editing and writing
//!
//! An edit is an ECS edit in HTML terms: a [`Value`] changed, `checked` set,
//! a node removed, a projection-only node added. Writing back is
//! re-projection: [`OoxmlRenderer`] answers a file's own media type by
//! writing every source node from its source components, and anything an
//! edit made with no source identity by the reverse of the mapping, new
//! content in its neighbour's style. An untouched file writes back render
//! identical, every part, relationship and attribute kept and only lexical
//! form free, and an edited one differs only where it was edited.
//!
//! # Formats
//!
//! | Format | Read as |
//! |---|---|
//! | Word | its headers, main document, footers and notes, the main document's tables numbered as a fill addresses them and captioned `<!-- t<n> -->` |
//! | Slide deck | each listed slide a `<section>` in presentation order headed `Slide <n>`, its shapes in reading order with their [`SourceOrder`] recorded, each text frame labelled by its kind, its tables, its SmartArt read from its data part, its pictures `<img>`s with their [`SlidePicture`], and its speaker notes an `<aside>`; [`OoxmlPlugin`] appends a triage of the deck in [`PostParseTree`] |
//! | Workbook | each worksheet a `<table>` captioned with its tab name, every cell a [`SheetCellAddress`] with its [`CellLocked`] and [`CellFormula`], shared strings resolved, a merged range's first cell spanning the rest, and every gap before a row's last cell a projection-only `<td>` |
//!
//! [`PageMeta`]: crate::prelude::PageMeta
//! [`PostParseTree`]: crate::prelude::PostParseTree
//! [`TableCellAddress`]: crate::prelude::TableCellAddress
//! [`SheetCellAddress`]: crate::prelude::SheetCellAddress
//! [`CellLocked`]: crate::prelude::CellLocked
//! [`CellFormula`]: crate::prelude::CellFormula
//! [`Element`]: beet_core::prelude::Element
//! [`Value`]: beet_core::prelude::Value
//! [`SourceElement`]: beet_core::prelude::SourceElement
//! [`SourceText`]: beet_core::prelude::SourceText
//! [`SourceOrder`]: beet_core::prelude::SourceOrder
//! [`SourcePart`]: beet_core::prelude::SourcePart
mod core_properties;
mod deck;
mod deck_report;
mod namespace;
mod ooxml_plugin;
mod package;
mod parser;
mod renderer;
mod source_tree;
mod word;
mod workbook;
pub use core_properties::*;
pub use deck::*;
pub use deck_report::DeckReported;
pub use namespace::*;
pub use ooxml_plugin::*;
pub use package::*;
pub use parser::*;
pub use renderer::*;
pub use word::*;
pub use workbook::*;
