//! What a parse into a world is handed, and what any parse may refuse with.
use crate::prelude::*;
use thiserror::Error;

/// A parse into a world: the bytes, the entity they land on and the path they
/// were read from, for [`FileSpan`] tracking. What
/// [`DocParsers::parse`] takes; a [`DocParser`] itself takes bytes alone.
///
/// Two lifetimes: `'w` for the world borrow inside [`EntityWorldMut`], and
/// `'a` for every other borrow, which must outlive the parse.
pub struct ParseContext<'a, 'w> {
	/// The entity to parse onto.
	pub entity: &'a mut EntityWorldMut<'w>,
	/// The typed bytes to parse.
	pub bytes: &'a MediaBytes,
	/// The source path, for [`FileSpan`] tracking.
	pub path: Option<WsPath>,
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
		}
	}

	/// Set the source path.
	pub fn with_path(mut self, path: WsPath) -> Self {
		self.path = Some(path);
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
	/// The format builds into a world as it parses, so it has no body to
	/// return: parse it through [`DocParsers::parse`].
	#[error("`{0}` parses into a world rather than to a scene body")]
	WorldOnly(MediaType),
	/// Any other parse failure.
	#[error("{0}")]
	Other(BevyError),
}

impl From<BevyError> for ParseError {
	fn from(err: BevyError) -> Self { ParseError::Other(err) }
}
