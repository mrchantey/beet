//! Edits in HTML terms over any document tree: a value changed, `checked`
//! set, a node removed, a projection-only node added.
//!
//! [`DocumentTreeEdit`] holds the edits that may span several nodes, each a
//! method answering what it touched, so the same edits fill a markdown form,
//! an HTML page or a Word file. A cell is addressed by its
//! [`TableCellAddress`], or a workbook's by its `SheetCellAddress` with the
//! `ooxml` feature, a checkbox by the label in its paragraph, a paragraph by
//! a text it carries; matching reads a no-break space as a space, since a
//! form's placeholders carry them.
//!
//! | Method | Does |
//! |---|---|
//! | [`set_cell_text`](DocumentTreeEdit::set_cell_text) | replaces a cell's paragraphs with one per line, the first keeping its own and its first words' look |
//! | [`append_cell_text`](DocumentTreeEdit::append_cell_text) | adds one paragraph per line after a cell's own |
//! | [`check_labeled_boxes`](DocumentTreeEdit::check_labeled_boxes) | checks every checkbox whose paragraph carries a label |
//! | [`remove_paragraphs`](DocumentTreeEdit::remove_paragraphs) | removes every paragraph carrying a text, a cell keeping one empty |
//! | [`replace_text`](DocumentTreeEdit::replace_text) | replaces a text inside the words that carry it, each piece keeping its look |
//!
//! An edit leaves what it wrote meaning only what HTML says, so a format's
//! renderer writes it by the reverse of its mapping, new content in its
//! neighbour's style, as `OoxmlRenderer` writes an Office file.
mod document_tree_edit;
pub use document_tree_edit::*;
