//! [`StructuredParser`], the self describing data formats.
//!
//! The structured formats are json, postcard and toml, each behind its own
//! feature, and DAG-CBOR where a record is the protocol. RON is not among
//! them, deliberately: every shape beet shares with atproto carries a
//! `$type` (a scene body names `org.beet.scene#body`), which is no field name
//! RON accepts, and beet does not shape its types around one format.
use beet_core::prelude::*;

/// Reads the structured formats this build has a feature for (json,
/// postcard, toml) as a scene body when the root object names one in its
/// `$type` (`org.beet.scene#body`), else as one entity holding the parsed
/// [`Value`], so a foreign json document is a value as a foreign Automerge
/// one would be.
///
/// Postcard is not self describing, so a postcard body is told from a
/// postcard value by the `$type` a body is always written with.
#[derive(Debug, Default, Clone)]
pub struct StructuredParser;

impl DocParser for StructuredParser {
	fn media_types(&self) -> Vec<MediaType> { MediaType::structured() }

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		_parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		if *bytes.media_type() == MediaType::Postcard {
			return match bytes.deserialize::<SceneBody>() {
				Ok(body) => body,
				Err(_) => Self::holding(bytes.deserialize()?)?,
			}
			.xmap(ParsedDocument::from)
			.xok();
		}
		let value = Self::deserialize::<Value>(bytes)?;
		if !SceneBody::is_body(&value) {
			return ParsedDocument::from(Self::holding(value)?).xok();
		}
		ParsedDocument {
			// read again as a body, which keeps the file's entity order where
			// a `Value`'s map is sorted
			body: Self::deserialize(bytes)?,
			spans: SourceMap::of_structured(bytes)?,
		}
		.xok()
	}
}

impl StructuredParser {
	/// A body of one entity holding `value`.
	fn holding(value: Value) -> Result<SceneBody> {
		SceneBody::root(EntityBody::default().with_typed(&value)?).xok()
	}

	/// `bytes` as a `T`, an error placed where the format's reader says it is.
	fn deserialize<T: DeserializeOwned>(bytes: &MediaBytes) -> Result<T> {
		bytes.media_type().deserialize_document(bytes)
	}
}

impl Installer for StructuredParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}
