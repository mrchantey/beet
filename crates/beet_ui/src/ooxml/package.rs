use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;
use ooxmlsdk::parts::header_part::HeaderPart;
use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk::parts::slide_part::SlidePart;
use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
use ooxmlsdk::schemas::opc_relationships::TargetMode;
use ooxmlsdk::sdk::SdkPackage;

/// The Office file a document tree was read from, on the tree's root: the
/// file as read, so every part the tree does not hold writes back as it was.
#[derive(Debug, Clone, Component)]
pub struct OoxmlPackage {
	/// The file as read.
	bytes: MediaBytes,
}

impl OoxmlPackage {
	/// The file `bytes` holds.
	pub(crate) fn new(bytes: MediaBytes) -> Self { Self { bytes } }

	/// The kind of file, as its media type.
	pub fn media_type(&self) -> &MediaType { self.bytes.media_type() }

	/// The file opened, to read or write its parts.
	pub fn open(&self) -> Result<OoxmlFile> { OoxmlFile::open(&self.bytes) }
}

/// An Office file opened: its package of parts, held by `ooxmlsdk`, which
/// opens from bytes and saves to them, so nothing is ever unzipped beside
/// it, and regenerates the content types and relationships with the entries
/// they were read with.
pub enum OoxmlFile {
	/// A Word file.
	Word(WordprocessingDocument),
	/// A slide deck.
	Deck(PresentationDocument),
}

/// Applies `$body` to the typed handle of every part kind a tree reads or a
/// writer writes, binding it as `$handle`; any other kind answers `$other`.
macro_rules! part_handle {
	($part:expr, $handle:ident => $body:expr, _ => $other:expr) => {
		match $part {
			PartRef::MainDocumentPart($handle) => $body,
			PartRef::HeaderPart($handle) => $body,
			PartRef::FooterPart($handle) => $body,
			PartRef::FootnotesPart($handle) => $body,
			PartRef::EndnotesPart($handle) => $body,
			PartRef::PresentationPart($handle) => $body,
			PartRef::SlidePart($handle) => $body,
			PartRef::SlideLayoutPart($handle) => $body,
			PartRef::NotesSlidePart($handle) => $body,
			PartRef::DiagramDataPart($handle) => $body,
			PartRef::ImagePart($handle) => $body,
			PartRef::CoreFilePropertiesPart($handle) => $body,
			_ => $other,
		}
	};
}

impl OoxmlFile {
	/// Opens the package `bytes` hold, a Word file or a deck by
	/// their media type.
	pub fn open(bytes: &MediaBytes) -> Result<Self> {
		let cursor = || std::io::Cursor::new(bytes.bytes().to_vec());
		match bytes.media_type() {
			MediaType::Docx => {
				Self::Word(WordprocessingDocument::new(cursor())?)
			}
			MediaType::Pptx => Self::Deck(PresentationDocument::new(cursor())?),
			other => bevybail!(
				"`{other}` is no Office file: expected a Word file or a slide deck"
			),
		}
		.xok()
	}

	/// A new Word file whose body is `body`, WordprocessingML with the `w:`,
	/// `w14:` and `r:` prefixes bound, ie for a test fixture.
	pub fn word(body: &str) -> Result<MediaBytes> {
		Self::word_with(body, WordParts::default())
	}

	/// [`word`](Self::word) with the parts `parts` adds beside its body.
	pub fn word_with(body: &str, parts: WordParts) -> Result<MediaBytes> {
		let mut file = WordprocessingDocument::create(Default::default());
		if let Some(properties) = parts.properties {
			let part = file.add_core_file_properties_part()?;
			part.set_data(
				&mut file,
				format!(
					"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
					 <cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\" xmlns:dcterms=\"{}\">\
					 {properties}</cp:coreProperties>",
					OoxmlNamespace::CORE_PROPERTIES,
					OoxmlNamespace::DUBLIN_CORE,
					OoxmlNamespace::DUBLIN_CORE_TERMS,
				),
			)?;
		}
		let main = file.add_main_document_part()?;
		main.set_data(
			&mut file,
			format!(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
				 <w:document xmlns:w=\"{}\" xmlns:w14=\"{}\" xmlns:r=\"{}\">\
				 <w:body>{body}</w:body></w:document>",
				OoxmlNamespace::WORD,
				OoxmlNamespace::WORD_2010,
				OoxmlNamespace::RELATIONSHIPS,
			),
		)?;
		for (id, target) in parts.links {
			main.add_hyperlink_relationship_with_mode(
				&mut file,
				*id,
				*target,
				TargetMode::External,
			)?;
		}
		if let Some(header) = parts.header {
			let part: HeaderPart =
				main.add_new_part(&mut file, "rIdHeader1")?;
			part.set_data(
				&mut file,
				format!(
					"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
					 <w:hdr xmlns:w=\"{}\">{header}</w:hdr>",
					OoxmlNamespace::WORD,
				),
			)?;
		}
		MediaBytes::new(MediaType::Docx, file.to_package_bytes()?).xok()
	}

	/// A new slide deck of `slides`, each a slide's shape tree inner
	/// PresentationML with the `p:`, `a:` and `r:` prefixes bound, on a
	/// 16:9 slide; ie for a test fixture.
	pub fn deck(slides: &[&str]) -> Result<MediaBytes> {
		let (presentation_ns, drawing, relationships) = (
			OoxmlNamespace::PRESENTATION,
			OoxmlNamespace::DRAWING,
			OoxmlNamespace::RELATIONSHIPS,
		);
		let mut file = PresentationDocument::create(Default::default());
		let presentation = file.add_presentation_part()?;
		let mut listed = String::new();
		for (index, shapes) in slides.iter().enumerate() {
			let id = format!("rIdSlide{}", index + 1);
			let slide: SlidePart = presentation.add_new_part(&mut file, &id)?;
			slide.set_data(
				&mut file,
				format!(
					"<p:sld xmlns:p=\"{presentation_ns}\" xmlns:a=\"{drawing}\" xmlns:r=\"{relationships}\">\
					 <p:cSld><p:spTree>{shapes}</p:spTree></p:cSld></p:sld>"
				),
			)?;
			listed.push_str(&format!(
				"<p:sldId id=\"{}\" r:id=\"{id}\"/>",
				256 + index
			));
		}
		presentation.set_data(
			&mut file,
			format!(
				"<p:presentation xmlns:p=\"{presentation_ns}\" xmlns:r=\"{relationships}\">\
				 <p:sldIdLst>{listed}</p:sldIdLst>\
				 <p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/>\
				 </p:presentation>"
			),
		)?;
		MediaBytes::new(MediaType::Pptx, file.to_package_bytes()?).xok()
	}

	/// The kind of file, as its media type.
	pub fn media_type(&self) -> MediaType {
		match self {
			Self::Word(_) => MediaType::Docx,
			Self::Deck(_) => MediaType::Pptx,
		}
	}

	/// The file's core properties, `docProps/core.xml`.
	pub fn core_properties(&self) -> Result<CoreProperties> {
		let part = self
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::CoreFilePropertiesPart(_)));
		CoreProperties::parse(
			part.as_ref()
				.map(|part| self.data(part))
				.transpose()?
				.flatten(),
		)
	}

	/// Every part of the package, wherever it is related from.
	pub fn parts(&self) -> Vec<PartRef> {
		match self {
			Self::Word(file) => all_parts(file),
			Self::Deck(file) => all_parts(file),
		}
	}

	/// The path of `part` in the package, ie `word/document.xml`.
	pub fn path(&self, part: &PartRef) -> Option<SmolStr> {
		match self {
			Self::Word(file) => part.path(file),
			Self::Deck(file) => part.path(file),
		}
		.map(|path| path.trim_start_matches('/').into())
	}

	/// The bytes of `part`, absent for a kind no tree reads.
	pub fn data(&self, part: &PartRef) -> Result<Option<&[u8]>> {
		macro_rules! read {
			($file:expr) => {
				part_handle!(part, handle => handle.try_data($file)?, _ => None)
			};
		}
		match self {
			Self::Word(file) => read!(file),
			Self::Deck(file) => read!(file),
		}
		.xok()
	}

	/// The bytes of the part at `path`.
	pub fn data_at(&self, path: &str) -> Result<Option<&[u8]>> {
		match self.part_at(path) {
			Some(part) => self.data(&part),
			None => Ok(None),
		}
	}

	/// The part at `path`.
	pub fn part_at(&self, path: &str) -> Option<PartRef> {
		self.parts()
			.into_iter()
			.find(|part| self.path(part).as_deref() == Some(path))
	}

	/// The text of the part at `path` read as XML, absent when the package
	/// has no such part.
	pub fn xml_at(&self, path: &str) -> Result<Option<Vec<BsxNode>>> {
		self.data_at(path)?
			.map(|bytes| Self::read_xml(bytes))
			.transpose()
	}

	/// A part's bytes as text read as XML, a UTF-16 part decoded.
	pub fn read_xml(bytes: &[u8]) -> Result<Vec<BsxNode>> {
		BsxNode::parse_document(&Self::decode(bytes)?, &BsxParseConfig::xml())
	}

	/// A part's bytes as text: UTF-8, or UTF-16 after its byte order mark.
	fn decode(bytes: &[u8]) -> Result<String> {
		let utf16 = |bytes: &[u8], from: fn([u8; 2]) -> u16| {
			let units = bytes
				.chunks_exact(2)
				.map(|pair| from([pair[0], pair[1]]))
				.collect::<Vec<_>>();
			String::from_utf16(&units)
		};
		match bytes {
			[0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes)?,
			[0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes)?,
			bytes => String::from_utf8(bytes.to_vec())?,
		}
		.xok()
	}

	/// The parts `part` relates, each by its relationship id.
	pub fn related(&self, part: &PartRef) -> Vec<(SmolStr, PartRef)> {
		macro_rules! related {
			($file:expr) => {
				part_handle!(part, handle => handle
					.parts($file)
					.map(|pair| (SmolStr::new(pair.relationship_id), pair.part))
					.collect(), _ => Vec::new())
			};
		}
		match self {
			Self::Word(file) => related!(file),
			Self::Deck(file) => related!(file),
		}
	}

	/// The hyperlink targets `part` relates, by relationship id.
	pub fn hyperlinks(&self, part: &PartRef) -> HashMap<SmolStr, SmolStr> {
		macro_rules! links {
			($file:expr) => {
				part_handle!(part, handle => handle
					.hyperlink_relationships($file)
					.map(|link| (SmolStr::new(link.id()), SmolStr::new(link.target())))
					.collect(), _ => HashMap::default())
			};
		}
		match self {
			Self::Word(file) => links!(file),
			Self::Deck(file) => links!(file),
		}
	}

	/// The id of an external hyperlink relationship from the part at `path`
	/// to `target`: one the part already holds, else a new one whose id no
	/// other relationship of the part has.
	pub fn hyperlink_to(
		&mut self,
		path: &str,
		target: &str,
	) -> Result<SmolStr> {
		let part = self
			.part_at(path)
			.ok_or_else(|| bevyhow!("the package has no part `{path}`"))?;
		if let Some(id) = self
			.hyperlinks(&part)
			.into_iter()
			.find(|(_, existing)| existing == target)
			.map(|(id, _)| id)
		{
			return Ok(id);
		}
		macro_rules! add {
			($file:expr) => {
				part_handle!(&part, handle => SmolStr::new(handle
					.add_hyperlink_relationship_auto_id($file, target, TargetMode::External)?
					.id()),
					_ => bevybail!("the part `{path}` holds no hyperlinks"))
			};
		}
		match self {
			Self::Word(file) => add!(file),
			Self::Deck(file) => add!(file),
		}
		.xok()
	}

	/// The image paths `part` relates, by relationship id.
	pub fn images(&self, part: &PartRef) -> HashMap<SmolStr, SmolStr> {
		self.related(part)
			.into_iter()
			.filter(|(_, part)| matches!(part, PartRef::ImagePart(_)))
			.filter_map(|(id, part)| Some((id, self.path(&part)?)))
			.collect()
	}

	/// Replaces the bytes of the part at `path`.
	pub fn set_data(&mut self, path: &str, bytes: Vec<u8>) -> Result {
		let part = self
			.part_at(path)
			.ok_or_else(|| bevyhow!("the package has no part `{path}`"))?;
		macro_rules! write {
			($file:expr) => {
				part_handle!(&part, handle => handle.set_data($file, bytes)?,
					_ => bevybail!("the part `{path}` is not one a tree writes"))
			};
		}
		match self {
			Self::Word(file) => write!(file),
			Self::Deck(file) => write!(file),
		}
		Ok(())
	}

	/// The package's bytes.
	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		match self {
			Self::Word(file) => file.to_package_bytes(),
			Self::Deck(file) => file.to_package_bytes(),
		}?
		.xok()
	}
}

/// What a fixture Word file holds beside its body, for
/// [`OoxmlFile::word_with`].
#[derive(Debug, Default, Clone)]
pub struct WordParts<'a> {
	/// The core properties, the inner elements of `docProps/core.xml` with
	/// the `cp:`, `dc:` and `dcterms:` prefixes bound, ie
	/// `<dc:creator>Ada</dc:creator>`.
	pub properties: Option<&'a str>,
	/// A header's inner WordprocessingML, its root binding only the `w:`
	/// prefix.
	pub header: Option<&'a str>,
	/// The main document's hyperlink relationships, each an id and its
	/// external target.
	pub links: &'a [(&'a str, &'a str)],
}

/// Every part of `file`, from its own relationships down.
fn all_parts(file: &impl SdkPackage) -> Vec<PartRef> {
	file.get_all_parts().collect()
}
