//! Error types for snippet parsing and serialization operations.

use beet_core::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// Result type alias for snippet operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during snippet parsing and serialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Failed to parse templates from a file.
	#[error("Failed to parse templates:\nPath: {path}\nDetails:\n{err}")]
	FileToTemplates {
		/// The path to the file that failed to parse.
		path: PathBuf,
		/// The error message.
		err: String,
	},
	/// Failed to collect language templates from a span.
	#[error("Failed to collect templates:\nSpan: {span}\nDetails:\n{err}")]
	CollectLangTemplates {
		/// The file span where the error occurred.
		span: FileSpan,
		/// The error message.
		err: String,
	},
	/// Failed to serialize to RON format.
	#[error("Failed to serialize:\nPath: {path}\nDetails:\n{err}")]
	SerializeRon {
		/// The path where serialization failed.
		path: PathBuf,
		/// The RON serialization error.
		err: ron::error::Error,
	},
	/// File system error wrapper.
	// Deliberately not `from` as some fs errors are FileToTemplates
	#[error("{0}")]
	File(FsError),
	/// CSS parsing error from lightningcss.
	#[error("Failed to parse css: {err}\nSpan: {span:?}")]
	LightningCss {
		/// Optional file span where the CSS error occurred.
		span: Option<FileSpan>,
		/// The CSS parsing error message.
		err: String,
	},
}

impl Error {
	/// Creates a RON serialization error.
	pub fn serialize_ron(
		path: impl AsRef<Path>,
		err: ron::error::Error,
	) -> Self {
		Self::SerializeRon {
			path: path.as_ref().to_path_buf(),
			err,
		}
	}

	/// Creates a file-to-templates parsing error.
	pub fn file_to_templates(
		path: impl AsRef<Path>,
		err: impl ToString,
	) -> Self {
		Self::FileToTemplates {
			path: path.as_ref().to_path_buf(),
			err: err.to_string(),
		}
	}

	/// Creates a language template collection error.
	pub fn collect_lang_templates(span: &FileSpan, err: impl ToString) -> Self {
		Self::CollectLangTemplates {
			span: span.clone(),
			err: err.to_string(),
		}
	}
}
