//! Office Open XML: Word files, workbooks and slide decks, over `ooxmlsdk`.
//!
//! `ooxmlsdk` holds a file's parts, opening from bytes and saving to bytes,
//! so a file opens from a [`Blob`](crate::prelude::Blob) and saves to one and
//! nothing is ever unzipped beside it. What is read or written inside a part
//! goes one of two ways:
//!
//! - **Typed**, where the shape is shallow and fixed: a [`Workbook`]'s sheets,
//!   rows, cells, styles, merges and shared strings are `ooxmlsdk`'s generated
//!   schema types, edited in place and serialized by it.
//! - **As an [`XmlTree`]**, where an operation is defined over descendants in
//!   document order: a [`WordDocument`]'s tables are numbered as they open,
//!   nested and boxed ones included, and its fill operations reach any
//!   paragraph, so its main part is an element tree written back on save. A
//!   slide deck's text is read the same way. `ooxmlsdk` models every element
//!   these touch, the `w14:checkbox` control included, but exposes no walk
//!   over them, and a hand-written one would enumerate every container type of
//!   every choice the schema allows.
//!
//! # Addresses
//!
//! A Word table cell is a [`TableCellAddress`], `t<table>r<row>c<cell>` from
//! 1; a workbook cell a [`SheetCellAddress`], `<sheet>!<A1>`. Both are what a
//! cells dump prints, one `| address | text |` row per cell, so a dump of a
//! blank form is the map a fill is written against.
//!
//! # Operations
//!
//! | Type | Operation | Does |
//! |---|---|---|
//! | [`WordDocument`] | [`cells`](WordDocument::cells) | every table cell in document order |
//! | | [`set_cell`](WordDocument::set_cell) | replaces a cell's paragraphs with one per line, in the cell's own paragraph and run style |
//! | | [`append_cell`](WordDocument::append_cell) | adds paragraphs after a cell's own |
//! | | [`check`](WordDocument::check) | ticks the checkbox controls whose paragraph carries a label: control checked, glyph ticked, bold and highlighted |
//! | | [`delete_paragraphs`](WordDocument::delete_paragraphs) | removes every paragraph carrying a text, a cell keeping one empty paragraph |
//! | | [`replace_text`](WordDocument::replace_text) | replaces a text inside the runs that carry it, keeping their formatting |
//! | | [`to_html`](WordDocument::to_html) | the file as HTML, what a form signals by look kept as markup |
//! | [`Workbook`] | [`unlocked_cells`](Workbook::unlocked_cells) | every cell a protected sheet lets a person type into, with its value |
//! | | [`set`](Workbook::set) | writes an unlocked cell, refusing a locked one or a formula |
//! | [`SlideDeck`] | [`to_html`](SlideDeck::to_html) | each slide's text, tables, pictures and speaker notes, and a triage of the deck |
//!
//! # Media
//!
//! A Word file and a slide deck are media like any other: `to_html` is their
//! transcode to the HTML every parser in beet produces, so `beet_ui`'s
//! `MediaParser` reads them into a page and its renderers answer them as
//! markdown, text or HTML. A cells dump is the one view that is not prose:
//! [`OoxmlCells`] answers it for a Word file or a workbook over the nearest
//! ancestor store.
mod cell_address;
mod core_properties;
mod html;
mod namespace;
mod ooxml_routes;
mod slides;
mod word;
mod word_html;
mod workbook;
mod xml_tree;
pub use cell_address::*;
pub use core_properties::*;
pub use namespace::*;
pub use ooxml_routes::*;
pub use slides::*;
pub use word::*;
pub use workbook::*;
pub use xml_tree::*;

/// A cell's text as a cells dump prints it: `(empty)` for nothing, a pipe
/// escaped so the row stays one row.
fn dump_text(text: &str) -> String {
	match text.is_empty() {
		true => "(empty)".into(),
		false => text.replace('|', "\\|"),
	}
}
