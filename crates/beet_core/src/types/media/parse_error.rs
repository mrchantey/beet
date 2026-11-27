//! [`ParseError`]: why a document did not parse.
use crate::prelude::*;
use thiserror::Error;

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
	/// The document did not read at `position`.
	#[error("{position}: {error}")]
	At {
		/// Where it did not read.
		position: SourcePosition,
		/// Why it did not.
		error: BevyError,
	},
	/// Any other parse failure.
	#[error("{0}")]
	Other(BevyError),
}

impl ParseError {
	/// This as an error in `source`, the document at `path` of its package:
	/// where a [`SourceError`] says it is, else the document alone.
	pub fn locate(self, source: &[u8], path: Option<&FwdPath>) -> Self {
		let Self::Other(error) = self else {
			return self;
		};
		let (offset, error) = SourceError::split(error);
		match (offset, path) {
			(Some(offset), path) => Self::At {
				position: SourcePosition::new(path, source, offset),
				error,
			},
			(None, Some(path)) => Self::At {
				position: SourcePosition::document(path.clone()),
				error,
			},
			(None, None) => Self::Other(error),
		}
	}

	/// This as the error a reader nested in another raises, its place kept.
	pub fn into_source(self) -> BevyError {
		match self {
			Self::Other(error) => error,
			other => other.into(),
		}
	}
}

impl From<BevyError> for ParseError {
	fn from(err: BevyError) -> Self { ParseError::Other(err) }
}
