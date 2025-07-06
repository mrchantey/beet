use beet_common::node::FileSpan;
use beet_utils::prelude::*;
// use beet_rsx::error::ParseError;
use std::path::Path;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;


#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Failed to parse templates:\nPath: {path}\nDetails:\n{err}")]
	FileToTemplates { path: PathBuf, err: String },
	#[error("Failed to collect templates:\nSpan: {span}\nDetails:\n{err}")]
	CollectLangTemplates { span: FileSpan, err: String },
	// #[error("Failed to parse templates:\nSpan: {span}\nDetails:\n{err}")]
	// Parse { span: FileSpan, err: ParseError },
	#[error("Failed to serialize:\nPath: {path}\nDetails:\n{err}")]
	SerializeRon {
		path: PathBuf,
		err: ron::error::Error,
	},
	// Deliberately not `from` as some fs errors are FileToTemplates
	#[error("{0}")]
	File(FsError),
	#[error("Failed to parse css: {err}\nSpan: {span:?}")]
	LightningCss { span: Option<FileSpan>, err: String },
}


impl Error {
	pub fn serialize_ron(
		path: impl AsRef<Path>,
		err: ron::error::Error,
	) -> Self {
		Self::SerializeRon {
			path: path.as_ref().to_path_buf(),
			err,
		}
	}
	pub fn file_to_templates(
		path: impl AsRef<Path>,
		err: impl ToString,
	) -> Self {
		Self::FileToTemplates {
			path: path.as_ref().to_path_buf(),
			err: err.to_string(),
		}
	}
	pub fn collect_lang_templates(span: &FileSpan, err: impl ToString) -> Self {
		Self::CollectLangTemplates {
			span: span.clone(),
			err: err.to_string(),
		}
	}
}
