use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;
use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk::parts::slide_part::SlidePart;
use ooxmlsdk::sdk::SdkPackage;

/// A slide deck opened: its package of parts, held by `ooxmlsdk`, which
/// opens from bytes, so nothing is ever unzipped beside it.
pub enum OoxmlFile {
	/// A slide deck.
	Deck(PresentationDocument),
}

/// Applies `$body` to the typed handle of every part kind a tree reads or a
/// writer writes, binding it as `$handle`; any other kind answers `$other`.
macro_rules! part_handle {
	($part:expr, $handle:ident => $body:expr, _ => $other:expr) => {
		match $part {
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
	/// Opens the deck `bytes` hold.
	pub fn open(bytes: &MediaBytes) -> Result<Self> {
		match bytes.media_type() {
			MediaType::Pptx => Self::Deck(PresentationDocument::new(
				std::io::Cursor::new(bytes.bytes().to_vec()),
			)?),
			other => bevybail!("`{other}` is no slide deck"),
		}
		.xok()
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
			Self::Deck(file) => all_parts(file),
		}
	}

	/// The path of `part` in the package, ie `word/document.xml`.
	pub fn path(&self, part: &PartRef) -> Option<SmolStr> {
		match self {
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
			Self::Deck(file) => read!(file),
		}
		.xok()
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
			Self::Deck(file) => links!(file),
		}
	}
}

/// Every part of `file`, from its own relationships down.
fn all_parts(file: &impl SdkPackage) -> Vec<PartRef> {
	file.get_all_parts().collect()
}
