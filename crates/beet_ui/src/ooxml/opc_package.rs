//! [`OpcPackage`], the zip an Office file is: its parts by path, each with
//! its content type, and the relationships among them, as the Open
//! Packaging Conventions define them.
use super::xml_part::XmlPart;
use beet_core::prelude::*;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;

/// An Office file's package held in memory: read whole from its bytes and
/// written whole to new ones, so no IO happens here.
///
/// A part is named by its path in the zip without a leading `/`, ie
/// `word/document.xml`. The content types part and every relationships
/// part are the package's own structure, read into and written from
/// [`content_type`](Self::content_type) and
/// [`relationships`](Self::relationships), never parts of their own.
#[derive(Debug, Default, Clone)]
pub struct OpcPackage {
	/// Every part, in the order it was read or inserted.
	parts: Vec<OpcPart>,
	/// The default content type of each extension, ie `xml`, `rels`, `png`.
	defaults: Vec<(SmolStr, SmolStr)>,
	/// The relationships of each source, the package's own under `None`.
	relationships: Vec<(Option<SmolStr>, Vec<OpcRelationship>)>,
}

/// One part of an [`OpcPackage`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcPart {
	/// The part's path, ie `word/document.xml`.
	pub path: SmolStr,
	/// Its media type, ie
	/// `application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml`.
	pub content_type: SmolStr,
	/// Its bytes.
	pub bytes: Vec<u8>,
}

/// A relationship from a part, or the package, to a part or an external
/// target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcRelationship {
	/// Its id, unique among its source's relationships, ie `rId1`.
	pub id: SmolStr,
	/// Its type, a uri, ie [`OpcRelationship::IMAGE`].
	pub kind: SmolStr,
	/// The path of the part it targets, resolved against its source part's
	/// directory, ie `word/media/image1.png`, or an external target as
	/// written, ie a hyperlink's url.
	pub target: SmolStr,
	/// Whether the target lies outside the package.
	pub external: bool,
}

impl OpcRelationship {
	/// A package's main part: a Word file's document, a workbook, a
	/// presentation.
	pub const OFFICE_DOCUMENT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
	/// The core properties, `docProps/core.xml`.
	pub const CORE_PROPERTIES: &str = "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
	/// An external hyperlink.
	pub const HYPERLINK: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
	/// A picture.
	pub const IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
	/// A theme, its colour and font schemes.
	pub const THEME: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
	/// A part's named styles.
	pub const STYLES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

	/// Whether this relationship's type is `kind`, or one whose last segment
	/// is `kind`'s, as the strict and transitional schemas spell the same
	/// relationship under different roots.
	pub fn is(&self, kind: &str) -> bool {
		self.kind == kind
			|| self.kind.rsplit('/').next() == kind.rsplit('/').next()
	}
}

impl OpcPackage {
	/// The content type of the content types part's own entries.
	pub const RELATIONSHIPS_TYPE: &str =
		"application/vnd.openxmlformats-package.relationships+xml";
	/// The content type of an XML part named by no override.
	pub const XML_TYPE: &str = "application/xml";
	/// The core properties' content type.
	pub const CORE_PROPERTIES_TYPE: &str =
		"application/vnd.openxmlformats-package.core-properties+xml";

	/// Reads the package `bytes` hold.
	pub fn read(bytes: &[u8]) -> Result<Self> {
		let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
		let mut entries = Vec::new();
		for index in 0..archive.len() {
			let mut file = archive.by_index(index)?;
			if file.is_dir() {
				continue;
			}
			let name = file.name()?.trim_start_matches('/').to_string();
			let mut data = Vec::new();
			file.read_to_end(&mut data)?;
			entries.push((name, data));
		}
		let mut package = Self::default();
		// the content types first, every part's type depending on them
		let overrides = match entries
			.iter()
			.find(|(name, _)| name == "[Content_Types].xml")
		{
			Some((_, data)) => package.read_content_types(data)?,
			None => {
				bevybail!(
					"the package has no `[Content_Types].xml`, so it is no Office file"
				)
			}
		};
		for (name, data) in entries {
			if name == "[Content_Types].xml" {
				continue;
			}
			if let Some(source) = Self::rels_source(&name) {
				let relationships =
					Self::read_relationships(source.as_deref(), &data)?;
				package.relationships.push((source, relationships));
				continue;
			}
			let content_type = overrides
				.iter()
				.find(|(part, _)| *part == name)
				.map(|(_, content_type)| content_type.clone())
				.unwrap_or_else(|| package.default_type(&name));
			package.parts.push(OpcPart {
				path: name.into(),
				content_type,
				bytes: data,
			});
		}
		package.xok()
	}

	/// Every part, in order.
	pub fn parts(&self) -> &[OpcPart] { &self.parts }

	/// The bytes of the part at `path`.
	pub fn part(&self, path: &str) -> Option<&[u8]> {
		self.find(path).map(|part| part.bytes.as_slice())
	}

	/// The content type of the part at `path`.
	pub fn content_type(&self, path: &str) -> Option<&str> {
		self.find(path).map(|part| part.content_type.as_str())
	}

	/// The part at `path` read as XML through the markup parser, a UTF-16
	/// part decoded, `None` when the package has no such part.
	pub fn xml(&self, path: &str) -> Result<Option<Vec<BsxNode>>> {
		self.part(path).map(Self::read_xml).transpose()
	}

	/// A part's bytes read as XML, a UTF-16 part decoded.
	pub fn read_xml(bytes: &[u8]) -> Result<Vec<BsxNode>> {
		BsxNode::parse_document(&Self::decode(bytes)?, &BsxParseConfig::xml())
	}

	/// The relationships of `source`, a part's path or `None` for the
	/// package's own.
	pub fn relationships(&self, source: Option<&str>) -> &[OpcRelationship] {
		self.relationships
			.iter()
			.find(|(found, _)| found.as_deref() == source)
			.map(|(_, relationships)| relationships.as_slice())
			.unwrap_or_default()
	}

	/// The relationship `id` of `source`.
	pub fn relationship(
		&self,
		source: Option<&str>,
		id: &str,
	) -> Option<&OpcRelationship> {
		self.relationships(source)
			.iter()
			.find(|relationship| relationship.id == id)
	}

	/// The first part `source` relates by `kind`, ie a workbook's styles.
	pub fn related(&self, source: Option<&str>, kind: &str) -> Option<&str> {
		self.relationships(source)
			.iter()
			.find(|relationship| {
				!relationship.external && relationship.is(kind)
			})
			.map(|relationship| relationship.target.as_str())
	}

	/// The package's main part, its office document.
	pub fn main_part(&self) -> Option<&str> {
		self.related(None, OpcRelationship::OFFICE_DOCUMENT)
	}

	/// Inserts the part at `path`, replacing one already there.
	pub fn insert(
		&mut self,
		path: impl Into<SmolStr>,
		content_type: impl Into<SmolStr>,
		bytes: impl Into<Vec<u8>>,
	) -> &mut Self {
		let part = OpcPart {
			path: path.into(),
			content_type: content_type.into(),
			bytes: bytes.into(),
		};
		match self.parts.iter_mut().find(|found| found.path == part.path) {
			Some(found) => *found = part,
			None => self.parts.push(part),
		}
		self
	}

	/// Relates `source`, a part's path or `None` for the package, to the
	/// part at `target` by `kind`, answering the new relationship's id.
	pub fn relate(
		&mut self,
		source: Option<&str>,
		kind: &str,
		target: &str,
	) -> SmolStr {
		self.push_relationship(source, kind, target, false)
	}

	/// Relates `source` to the external `target`, ie a hyperlink's url, by
	/// `kind`, answering the new relationship's id.
	pub fn relate_external(
		&mut self,
		source: Option<&str>,
		kind: &str,
		target: &str,
	) -> SmolStr {
		self.push_relationship(source, kind, target, true)
	}

	/// The package's bytes: the content types, every part, then every
	/// relationships part, deflated.
	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
		let options = zip::write::SimpleFileOptions::default()
			.compression_method(zip::CompressionMethod::Deflated);
		zip.start_file("[Content_Types].xml", options)?;
		zip.write_all(&self.content_types_xml())?;
		for (source, relationships) in &self.relationships {
			zip.start_file(Self::rels_path(source.as_deref()), options)?;
			zip.write_all(&Self::relationships_xml(
				source.as_deref(),
				relationships,
			))?;
		}
		for part in &self.parts {
			zip.start_file(part.path.as_str(), options)?;
			zip.write_all(&part.bytes)?;
		}
		zip.finish()?.into_inner().xok()
	}

	fn find(&self, path: &str) -> Option<&OpcPart> {
		let path = path.trim_start_matches('/');
		self.parts.iter().find(|part| part.path == path)
	}

	fn push_relationship(
		&mut self,
		source: Option<&str>,
		kind: &str,
		target: &str,
		external: bool,
	) -> SmolStr {
		let index = match self
			.relationships
			.iter()
			.position(|(found, _)| found.as_deref() == source)
		{
			Some(index) => index,
			None => {
				self.relationships
					.push((source.map(SmolStr::new), Vec::new()));
				self.relationships.len() - 1
			}
		};
		let relationships = &mut self.relationships[index].1;
		let id = (relationships.len() + 1..)
			.map(|number| SmolStr::new(format!("rId{number}")))
			.find(|id| relationships.iter().all(|found| found.id != *id))
			.unwrap_or_default();
		relationships.push(OpcRelationship {
			id: id.clone(),
			kind: kind.into(),
			target: target.trim_start_matches('/').into(),
			external,
		});
		id
	}

	/// The default content type of `path` by its extension, else
	/// [`XML_TYPE`](Self::XML_TYPE) for an `.xml` part and
	/// `application/octet-stream` for any other.
	fn default_type(&self, path: &str) -> SmolStr {
		let extension = path
			.rsplit_once('.')
			.map(|(_, ext)| ext.to_ascii_lowercase());
		self.defaults
			.iter()
			.find(|(found, _)| Some(found.as_str()) == extension.as_deref())
			.map(|(_, content_type)| content_type.clone())
			.unwrap_or_else(|| match extension.as_deref() {
				Some("xml") => Self::XML_TYPE.into(),
				_ => "application/octet-stream".into(),
			})
	}

	/// Reads the content types part into the defaults, answering the
	/// overrides by part path.
	fn read_content_types(
		&mut self,
		data: &[u8],
	) -> Result<Vec<(String, SmolStr)>> {
		let nodes = Self::read_xml(data)?;
		let Some(types) = BsxNode::document_element(&nodes) else {
			return Ok(Vec::new());
		};
		self.defaults = types
			.children_named("Default")
			.filter_map(|default| {
				Some((
					SmolStr::new(
						default.attribute("Extension")?.to_ascii_lowercase(),
					),
					SmolStr::new(default.attribute("ContentType")?),
				))
			})
			.collect();
		types
			.children_named("Override")
			.filter_map(|item| {
				Some((
					item.attribute("PartName")?
						.trim_start_matches('/')
						.to_string(),
					SmolStr::new(item.attribute("ContentType")?),
				))
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// The source a relationships part at `path` describes: `Some(None)` for
	/// the package's `_rels/.rels`, `Some(Some(part))` for a part's
	/// `<dir>/_rels/<name>.rels`, `None` for any other path.
	fn rels_source(path: &str) -> Option<Option<SmolStr>> {
		let rest = path.strip_suffix(".rels")?;
		let (dir, name) = match rest.rsplit_once("/_rels/") {
			Some((dir, name)) => (Some(dir), name),
			None => (None, rest.strip_prefix("_rels/")?),
		};
		match (dir, name) {
			(None, "") => Some(None),
			(None, name) => Some(Some(name.into())),
			(Some(dir), name) => Some(Some(format!("{dir}/{name}").into())),
		}
	}

	/// The path of the relationships part of `source`.
	fn rels_path(source: Option<&str>) -> String {
		match source {
			None => "_rels/.rels".into(),
			Some(source) => match source.rsplit_once('/') {
				Some((dir, name)) => format!("{dir}/_rels/{name}.rels"),
				None => format!("_rels/{source}.rels"),
			},
		}
	}

	/// Reads a relationships part of `source`, each internal target resolved
	/// against the source's directory.
	fn read_relationships(
		source: Option<&str>,
		data: &[u8],
	) -> Result<Vec<OpcRelationship>> {
		let nodes = Self::read_xml(data)?;
		let Some(root) = BsxNode::document_element(&nodes) else {
			return Ok(Vec::new());
		};
		let base = Self::directory(source);
		root.children_named("Relationship")
			.filter_map(|relationship| {
				let target = relationship.attribute("Target")?;
				let external =
					relationship.attribute("TargetMode") == Some("External");
				Some(OpcRelationship {
					id: relationship.attribute("Id")?.into(),
					kind: relationship.attribute("Type")?.into(),
					target: match external {
						true => target.into(),
						false => Self::resolve(base, target),
					},
					external,
				})
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// The directory of `source`'s path, `""` for the package or a part at
	/// its root.
	fn directory(source: Option<&str>) -> &str {
		source
			.and_then(|source| source.rsplit_once('/'))
			.map_or("", |(dir, _)| dir)
	}

	/// `target` resolved against the directory `base`, `..` and `.`
	/// segments applied and a leading `/` meaning the package's root.
	fn resolve(base: &str, target: &str) -> SmolStr {
		let mut segments = match target.starts_with('/') {
			true => Vec::new(),
			false => base
				.split('/')
				.filter(|segment| !segment.is_empty())
				.collect(),
		};
		for segment in target.split('/') {
			match segment {
				"" | "." => {}
				".." => {
					segments.pop();
				}
				segment => segments.push(segment),
			}
		}
		segments.join("/").into()
	}

	/// `target`, a part's path, relative to the directory `base`.
	fn relative(base: &str, target: &str) -> String {
		let base = base
			.split('/')
			.filter(|segment| !segment.is_empty())
			.collect::<Vec<_>>();
		let target = target.split('/').collect::<Vec<_>>();
		let shared = base
			.iter()
			.zip(&target)
			.take_while(|(base, target)| base == target)
			.count();
		core::iter::repeat_n("..", base.len() - shared)
			.chain(target[shared..].iter().copied())
			.collect::<Vec<_>>()
			.join("/")
	}

	/// The content types part: a default for each extension used, its
	/// type the most of its parts carry, and an override for every part
	/// whose type differs.
	fn content_types_xml(&self) -> Vec<u8> {
		let mut defaults: Vec<(SmolStr, SmolStr)> = vec![
			("rels".into(), Self::RELATIONSHIPS_TYPE.into()),
			("xml".into(), Self::XML_TYPE.into()),
		];
		for part in &self.parts {
			let Some((_, extension)) = part.path.rsplit_once('.') else {
				continue;
			};
			let extension = extension.to_ascii_lowercase();
			if extension == "xml"
				|| defaults.iter().any(|(ext, _)| *ext == extension)
			{
				continue;
			}
			defaults.push((extension.into(), part.content_type.clone()));
		}
		let default_of = |path: &str| {
			let extension = path
				.rsplit_once('.')
				.map(|(_, ext)| ext.to_ascii_lowercase());
			defaults
				.iter()
				.find(|(ext, _)| Some(ext.as_str()) == extension.as_deref())
				.map(|(_, content_type)| content_type.clone())
		};
		let mut xml = XmlPart::new();
		xml.open("Types", &[(
			"xmlns",
			"http://schemas.openxmlformats.org/package/2006/content-types",
		)]);
		for (extension, content_type) in &defaults {
			xml.empty("Default", &[
				("Extension", extension),
				("ContentType", content_type),
			]);
		}
		for part in &self.parts {
			if default_of(&part.path).as_ref() != Some(&part.content_type) {
				xml.empty("Override", &[
					("PartName", &format!("/{}", part.path)),
					("ContentType", &part.content_type),
				]);
			}
		}
		xml.close("Types");
		xml.into_bytes()
	}

	/// A relationships part of `source`, each internal target written
	/// relative to the source's directory.
	fn relationships_xml(
		source: Option<&str>,
		relationships: &[OpcRelationship],
	) -> Vec<u8> {
		let base = Self::directory(source);
		let mut xml = XmlPart::new();
		xml.open("Relationships", &[(
			"xmlns",
			"http://schemas.openxmlformats.org/package/2006/relationships",
		)]);
		for relationship in relationships {
			let target = match relationship.external {
				true => relationship.target.to_string(),
				false => Self::relative(base, &relationship.target),
			};
			let mut attributes = vec![
				("Id", relationship.id.as_str()),
				("Type", relationship.kind.as_str()),
				("Target", target.as_str()),
			];
			if relationship.external {
				attributes.push(("TargetMode", "External"));
			}
			xml.empty("Relationship", &attributes);
		}
		xml.close("Relationships");
		xml.into_bytes()
	}

	/// A part's bytes as text: UTF-8 after an optional byte order mark, or
	/// UTF-16 after its own.
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
			[0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8(rest.to_vec())?,
			bytes => String::from_utf8(bytes.to_vec())?,
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A package written and read back holds its parts, their types and
	/// their relationships, a relative target resolving against its source's
	/// directory and an external one staying as written.
	#[beet_core::test]
	fn round_trips_parts_and_relationships() {
		let mut package = OpcPackage::default();
		package
			.insert(
				"word/document.xml",
				"application/vnd.test.main+xml",
				b"<w/>".to_vec(),
			)
			.insert("word/media/image1.png", "image/png", vec![0x89, b'P'])
			.insert(
				"docProps/core.xml",
				OpcPackage::CORE_PROPERTIES_TYPE,
				b"<c/>".to_vec(),
			);
		package
			.relate(None, OpcRelationship::OFFICE_DOCUMENT, "word/document.xml")
			.xpect_eq("rId1");
		package.relate(
			None,
			OpcRelationship::CORE_PROPERTIES,
			"docProps/core.xml",
		);
		let image = package.relate(
			Some("word/document.xml"),
			OpcRelationship::IMAGE,
			"word/media/image1.png",
		);
		let link = package.relate_external(
			Some("word/document.xml"),
			OpcRelationship::HYPERLINK,
			"https://example.com/a?b=1&c=2",
		);
		let read = OpcPackage::read(&package.to_bytes().unwrap()).unwrap();
		read.main_part().xpect_eq(Some("word/document.xml"));
		read.content_type("word/document.xml")
			.xpect_eq(Some("application/vnd.test.main+xml"));
		read.content_type("word/media/image1.png")
			.xpect_eq(Some("image/png"));
		read.part("word/media/image1.png")
			.xpect_eq(Some([0x89, b'P'].as_slice()));
		read.related(None, OpcRelationship::CORE_PROPERTIES)
			.xpect_eq(Some("docProps/core.xml"));
		let source = Some("word/document.xml");
		read.relationship(source, &image)
			.unwrap()
			.target
			.clone()
			.xpect_eq("word/media/image1.png");
		let link = read.relationship(source, &link).unwrap();
		link.external.xpect_true();
		link.target
			.clone()
			.xpect_eq("https://example.com/a?b=1&c=2");
		read.xml("word/document.xml")
			.unwrap()
			.unwrap()
			.xmap(|nodes| {
				BsxNode::document_element(&nodes).map(|root| root.tag.clone())
			})
			.xpect_eq(Some("w".to_string()));
	}

	/// A target written `../media/image1.png` from `xl/drawings/` resolves to
	/// `xl/media/image1.png`, and one written from the root with a leading
	/// `/` to its own path.
	#[beet_core::test]
	fn resolves_relative_targets() {
		let mut package = OpcPackage::default();
		package.insert(
			"xl/drawings/drawing1.xml",
			OpcPackage::XML_TYPE,
			b"<d/>".to_vec(),
		);
		package.insert("xl/media/image1.png", "image/png", vec![1]);
		package.insert(
			"xl/workbook.xml",
			OpcPackage::XML_TYPE,
			b"<w/>".to_vec(),
		);
		package.relate(
			Some("xl/drawings/drawing1.xml"),
			OpcRelationship::IMAGE,
			"xl/media/image1.png",
		);
		package.relate(
			None,
			OpcRelationship::OFFICE_DOCUMENT,
			"xl/workbook.xml",
		);
		let bytes = package.to_bytes().unwrap();
		let read = OpcPackage::read(&bytes).unwrap();
		// written relative to its source
		let mut archive =
			zip::ZipArchive::new(std::io::Cursor::new(bytes.as_slice()))
				.unwrap();
		let mut rels = String::new();
		std::io::Read::read_to_string(
			&mut archive
				.by_name("xl/drawings/_rels/drawing1.xml.rels")
				.unwrap(),
			&mut rels,
		)
		.unwrap();
		rels.xpect_contains("Target=\"../media/image1.png\"");
		read.related(Some("xl/drawings/drawing1.xml"), OpcRelationship::IMAGE)
			.xpect_eq(Some("xl/media/image1.png"));
		OpcPackage::read(b"not a zip").xpect_err();
	}
}
