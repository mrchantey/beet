//! [`WordFixture`], a synthetic Word file for tests, beet's own and a
//! downstream's, so no real document is ever committed as a fixture.
use super::office_picture::OfficePicture;
use crate::prelude::*;
use beet_core::prelude::*;

/// A synthetic Word file built over [`OpcPackage`]: a body with every
/// namespace a reader reads bound, its styles, numbering, notes, header,
/// footer, relationships and core properties.

#[derive(Default)]
pub struct WordFixture {
	body: String,
	styles: Option<String>,
	numbering: Option<String>,
	footnotes: Vec<String>,
	endnotes: Vec<String>,
	header: Option<String>,
	footer: Option<String>,
	/// Each external hyperlink's id and target, in id order.
	links: Vec<(String, String)>,
	/// Each picture's id, content type and bytes, after the links.
	images: Vec<(String, String, Vec<u8>)>,
	properties: Option<String>,
}

impl WordFixture {
	/// The prefixes a fixture's parts bind.
	const NAMESPACES: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
		xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
		xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
		xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
		xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\" \
		xmlns:wps=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\" \
		xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
		xmlns:w14=\"http://schemas.microsoft.com/office/word/2010/wordml\" \
		xmlns:v=\"urn:schemas-microsoft-com:vml\"";

	/// A file whose body is `body`.
	pub fn new(body: &str) -> Self {
		Self {
			body: body.into(),
			..default()
		}
	}

	/// Sets the styles part's inner elements.
	pub fn with_styles(mut self, xml: &str) -> Self {
		self.styles = Some(xml.into());
		self
	}

	/// Sets the numbering part's inner elements.
	pub fn with_numbering(mut self, xml: &str) -> Self {
		self.numbering = Some(xml.into());
		self
	}

	/// Adds a footnote of `xml` blocks, its id the count before it plus 1.
	pub fn with_footnote(mut self, xml: &str) -> Self {
		self.footnotes.push(xml.into());
		self
	}

	/// Adds an endnote of `xml` blocks.
	pub fn with_endnote(mut self, xml: &str) -> Self {
		self.endnotes.push(xml.into());
		self
	}

	/// Sets the default header's blocks.
	pub fn with_header(mut self, xml: &str) -> Self {
		self.header = Some(xml.into());
		self
	}

	/// Sets the default footer's blocks.
	pub fn with_footer(mut self, xml: &str) -> Self {
		self.footer = Some(xml.into());
		self
	}

	/// Relates the document to the external `target` by `id`, the ids
	/// counted `rId1`, `rId2`, .. across links then pictures.
	pub fn with_link(mut self, id: &str, target: &str) -> Self {
		self.links.push((id.into(), target.into()));
		self
	}

	/// Relates the document to a picture of `content_type` by `id`.
	pub fn with_image(
		mut self,
		id: &str,
		content_type: &str,
		bytes: Vec<u8>,
	) -> Self {
		self.images.push((id.into(), content_type.into(), bytes));
		self
	}

	/// Sets the core properties' inner elements, `dc:` bound.
	pub fn with_properties(mut self, xml: &str) -> Self {
		self.properties = Some(xml.into());
		self
	}

	/// The file's bytes.
	pub fn bytes(self) -> MediaBytes {
		let namespaces = Self::NAMESPACES;
		let main = "word/document.xml";
		let mut package = OpcPackage::default();
		// the fixture names its own ids, so they are added first, in order
		for (id, target) in &self.links {
			let added = package.relate_external(
				Some(main),
				OpcRelationship::HYPERLINK,
				target,
			);
			assert_eq!(&added, id, "fixture ids are `rId1`, `rId2`, ..");
		}
		for (index, (id, content_type, bytes)) in self.images.iter().enumerate()
		{
			let path = format!(
				"word/media/image{}.{}",
				index + 1,
				OfficePicture::extension(&MediaType::from_content_type(
					content_type
				))
			);
			package.insert(path.as_str(), content_type.as_str(), bytes.clone());
			let added =
				package.relate(Some(main), OpcRelationship::IMAGE, &path);
			assert_eq!(&added, id, "fixture ids are `rId1`, `rId2`, ..");
		}
		let mut section = String::new();
		for (xml, name, root, reference, kind) in [
			(
				&self.header,
				"header",
				"w:hdr",
				"w:headerReference",
				OpcRelationship::HEADER,
			),
			(
				&self.footer,
				"footer",
				"w:ftr",
				"w:footerReference",
				OpcRelationship::FOOTER,
			),
		] {
			let Some(xml) = xml else {
				continue;
			};
			let path = format!("word/{name}1.xml");
			package.insert(
				path.as_str(),
				OpcPackage::XML_TYPE,
				format!("<{root} {namespaces}>{xml}</{root}>"),
			);
			let id = package.relate(Some(main), kind, &path);
			section.push_str(&format!(
				"<{reference} w:type=\"default\" r:id=\"{id}\"/>"
			));
		}
		package.insert(
			main,
			"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
			format!(
				"<w:document {namespaces}><w:body>{}<w:sectPr>{section}</w:sectPr></w:body></w:document>",
				self.body
			),
		);
		package.relate(None, OpcRelationship::OFFICE_DOCUMENT, main);
		for (xml, path, root, kind) in [
			(
				&self.styles,
				"word/styles.xml",
				"w:styles",
				OpcRelationship::STYLES,
			),
			(
				&self.numbering,
				"word/numbering.xml",
				"w:numbering",
				OpcRelationship::NUMBERING,
			),
		] {
			if let Some(xml) = xml {
				package.insert(
					path,
					OpcPackage::XML_TYPE,
					format!("<{root} {namespaces}>{xml}</{root}>"),
				);
				package.relate(Some(main), kind, path);
			}
		}
		for (notes, element, path, kind) in [
			(
				&self.footnotes,
				"footnote",
				"word/footnotes.xml",
				OpcRelationship::FOOTNOTES,
			),
			(
				&self.endnotes,
				"endnote",
				"word/endnotes.xml",
				OpcRelationship::ENDNOTES,
			),
		] {
			if notes.is_empty() {
				continue;
			}
			let mut xml = format!(
				"<w:{element}s {namespaces}><w:{element} w:type=\"separator\" w:id=\"-1\"><w:p><w:r><w:separator/></w:r></w:p></w:{element}>"
			);
			for (index, note) in notes.iter().enumerate() {
				xml.push_str(&format!(
					"<w:{element} w:id=\"{}\">{note}</w:{element}>",
					index + 1
				));
			}
			xml.push_str(&format!("</w:{element}s>"));
			package.insert(path, OpcPackage::XML_TYPE, xml);
			package.relate(Some(main), kind, path);
		}
		if let Some(properties) = self.properties {
			package.insert(
				DocMeta::CORE_PROPERTIES_PATH,
				OpcPackage::CORE_PROPERTIES_TYPE,
				format!(
					"<cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\">{properties}</cp:coreProperties>",
					OoxmlNamespace::CORE_PROPERTIES,
					OoxmlNamespace::DUBLIN_CORE,
				),
			);
			package.relate(
				None,
				OpcRelationship::CORE_PROPERTIES,
				DocMeta::CORE_PROPERTIES_PATH,
			);
		}
		MediaBytes::new(MediaType::Docx, package.to_bytes().unwrap())
	}
}
