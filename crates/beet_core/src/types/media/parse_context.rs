//! What a parse into a world is handed, and what any parse may refuse with.
use crate::prelude::*;
use thiserror::Error;

/// A parse into a world: the bytes, the entity they land on and the package
/// path they were read from. What [`DocParsers::parse`] takes; a [`DocParser`]
/// itself takes bytes alone.
///
/// Two lifetimes: `'w` for the world borrow inside [`EntityWorldMut`], and
/// `'a` for every other borrow, which must outlive the parse.
pub struct ParseContext<'a, 'w> {
	/// The entity to parse onto.
	pub entity: &'a mut EntityWorldMut<'w>,
	/// The typed bytes to parse.
	pub bytes: &'a MediaBytes,
	/// The document's path in its package, which a [`DocPath`] it holds
	/// resolves from.
	pub path: Option<RelPath>,
	/// The component the document's unsectioned metadata declares, the
	/// registry's own by default ([`DocParsers::metadata_type`]).
	pub metadata_type: Option<SmolStr>,
}

impl<'a, 'w> ParseContext<'a, 'w> {
	/// Parse `bytes` onto `entity`.
	pub fn new(
		entity: &'a mut EntityWorldMut<'w>,
		bytes: &'a MediaBytes,
	) -> Self {
		Self {
			entity,
			bytes,
			path: None,
			metadata_type: None,
		}
	}

	/// Parse as the document at `path` of its package.
	pub fn with_path(mut self, path: RelPath) -> Self {
		self.path = Some(path);
		self
	}

	/// Declare the document's unsectioned metadata as the component
	/// `metadata_type` names.
	pub fn with_metadata_type(
		mut self,
		metadata_type: impl Into<SmolStr>,
	) -> Self {
		self.metadata_type = Some(metadata_type.into());
		self
	}
}

/// Why a document did not parse.
#[derive(Debug, Error)]
pub enum ParseError {
	/// No parser reads the media type.
	#[error("unsupported media type `{unsupported}`, supported: {supported:?}")]
	UnsupportedType {
		/// The unsupported media type.
		unsupported: MediaType,
		/// The types that are supported.
		supported: Vec<MediaType>,
	},
	/// Any other parse failure.
	#[error("{0}")]
	Other(BevyError),
}

impl From<BevyError> for ParseError {
	fn from(err: BevyError) -> Self { ParseError::Other(err) }
}
