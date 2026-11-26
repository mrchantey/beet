//! The XML namespaces Office Open XML parts are written in.

/// The namespaces an Office part's elements and attributes live in, each by
/// the uri it is matched on, since a file may bind any prefix to it; the
/// prefix in each item is only the one Office writes.
pub struct OoxmlNamespace;

impl OoxmlNamespace {
	/// XML's own, `xml:`, bound in every document: `xml:space`.
	pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
	/// WordprocessingML, a Word file's body, `w:`: `w:p`, `w:tbl`, `w:r`.
	pub const WORD: &str =
		"http://schemas.openxmlformats.org/wordprocessingml/2006/main";
	/// Word 2010's additions, `w14:`, the `w14:checkbox` content control among
	/// them.
	pub const WORD_2010: &str =
		"http://schemas.microsoft.com/office/word/2010/wordml";
	/// DrawingML's placement of a drawing in a Word file, `wp:`: `wp:inline`,
	/// `wp:docPr`.
	pub const WORD_DRAWING: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
	/// PresentationML, a deck's slides, `p:`: `p:sld`, `p:sp`, `p:ph`.
	pub const PRESENTATION: &str =
		"http://schemas.openxmlformats.org/presentationml/2006/main";
	/// DrawingML, shared by every kind of file, `a:`: text bodies, tables and
	/// pictures, `a:p`, `a:tbl`, `a:blip`.
	pub const DRAWING: &str =
		"http://schemas.openxmlformats.org/drawingml/2006/main";
	/// DrawingML diagrams, `dgm:`, the data part SmartArt's text lives in.
	pub const DIAGRAM: &str =
		"http://schemas.openxmlformats.org/drawingml/2006/diagram";
	/// Relationships, `r:`, how an element names another part: `r:id`,
	/// `r:embed`.
	pub const RELATIONSHIPS: &str =
		"http://schemas.openxmlformats.org/officeDocument/2006/relationships";
	/// SpreadsheetML, a workbook's sheets, cells, styles and shared strings,
	/// written unprefixed: `worksheet`, `c`, `si`.
	pub const SPREADSHEET: &str =
		"http://schemas.openxmlformats.org/spreadsheetml/2006/main";
	/// Markup compatibility, `mc:`: `mc:AlternateContent`, a newer form and
	/// its fallback.
	pub const COMPATIBILITY: &str =
		"http://schemas.openxmlformats.org/markup-compatibility/2006";
	/// Dublin Core, `dc:`, the core properties' title and creator.
	pub const DUBLIN_CORE: &str = "http://purl.org/dc/elements/1.1/";
	/// Dublin Core terms, `dcterms:`, the core properties' dates.
	pub const DUBLIN_CORE_TERMS: &str = "http://purl.org/dc/terms/";
	/// The core properties part's own, `cp:`: `cp:lastModifiedBy`,
	/// `cp:revision`.
	pub const CORE_PROPERTIES: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
}
