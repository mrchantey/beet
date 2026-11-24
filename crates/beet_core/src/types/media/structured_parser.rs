//! [`StructuredParser`], the self describing data formats.
//!
//! The structured formats are json, postcard and toml, each behind its own
//! feature, and DAG-CBOR where a record is the protocol. RON is not among
//! them, deliberately: every shape beet shares with atproto carries a
//! `$type` (a scene body names `org.beet.scene#body`), which is no field name
//! RON accepts, and beet does not shape its types around one format.
use crate::prelude::*;

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
	fn media_types(&self) -> Vec<MediaType> {
		[
			cfg!(feature = "json").then_some(MediaType::Json),
			cfg!(feature = "postcard").then_some(MediaType::Postcard),
			cfg!(feature = "toml").then_some(MediaType::Toml),
		]
		.into_iter()
		.flatten()
		.collect()
	}

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		_parsers: &DocParsers,
	) -> Result<SceneBody, ParseError> {
		if *bytes.media_type() == MediaType::Postcard {
			return match bytes.deserialize::<SceneBody>() {
				Ok(body) => body.xok(),
				Err(_) => Self::holding(bytes.deserialize()?),
			};
		}
		let value = bytes.deserialize::<Value>()?;
		match SceneBody::is_body(&value) {
			// read again as a body, which keeps the file's entity order where
			// a `Value`'s map is sorted
			true => bytes.deserialize::<SceneBody>()?.xok(),
			false => Self::holding(value),
		}
	}
}

impl StructuredParser {
	/// A body of one entity holding `value`.
	fn holding(value: Value) -> Result<SceneBody, ParseError> {
		SceneBody::root(EntityBody::default().with_typed(&value)?).xok()
	}
}

impl Installer for StructuredParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}
