//! File system error types.
//!
//! This module provides [`FsError`], an error type for file system operations
//! that includes the path that caused the error, making debugging easier.

use std::io;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using [`FsError`].
pub type FsResult<T = ()> = std::result::Result<T, FsError>;


/// An file system error that includes the path that caused the error.
///
/// Unlike [`std::io::Error`], this error type always includes the path,
/// making it easier to debug file system issues.
#[derive(Debug, Error)]
pub enum FsError {
	/// A file was not found at the specified path.
	#[error("Fs Error - File Not Found\nPath: {path}")]
	FileNotFound {
		/// The path that was not found.
		path: PathBuf,
	},
	/// A directory was not found at the specified path.
	#[error("Fs Error - Dir Not Found\nPath: {path}")]
	DirNotFound {
		/// The path that was not found.
		path: PathBuf,
	},
	/// A file or directory already exists at the specified path.
	#[error("Fs Error - Already Exists\nPath: {path}")]
	AlreadyExists {
		/// The path that already exists.
		path: PathBuf,
	},
	/// Catch-all error for IO errors that are not [`io::ErrorKind::NotFound`].
	#[error("Fs Error - IO\nPath: {path}\nError: {err}")]
	Io {
		/// The path involved in the operation.
		path: PathBuf,
		/// The underlying IO error.
		err: io::Error,
	},
	/// An error occurred while reading a child entry during directory iteration.
	///
	/// [`fs::read_dir`](std::fs::read_dir) may succeed but reading one child may fail,
	/// in which case we don't know the child path.
	#[error("Fs Error - Child IO\nParent: {parent}\nError: {err}")]
	ChildIo {
		/// The parent directory being read.
		parent: PathBuf,
		/// The underlying IO error.
		err: io::Error,
	},
	/// The path was invalid (e.g., contains invalid characters).
	#[error("Fs Error - Invalid Path\nPath: {path}\nError: {err}")]
	InvalidPath {
		/// The invalid path.
		path: PathBuf,
		/// A description of why the path is invalid.
		err: String,
	},
	/// A generic error with a custom message.
	#[error("Fs Error\nPath: {path}\nError: {err}")]
	Other {
		/// The path involved in the operation.
		path: PathBuf,
		/// The error message.
		err: String,
	},
}

impl FsError {
	/// Asserts that the path is a directory, returning an error if not.
	pub fn assert_dir(path: impl AsRef<Path>) -> FsResult {
		if !path.as_ref().is_dir() {
			Err(FsError::DirNotFound {
				path: path.as_ref().into(),
			})
		} else {
			Ok(())
		}
	}


	/// Creates an [`FsError`] from an [`io::Error`], inferring the error type from the path.
	pub fn io(path: impl AsRef<Path>, e: io::Error) -> Self {
		let path: PathBuf = path.as_ref().into();
		match (e.kind(), path.is_dir()) {
			(io::ErrorKind::NotFound, true) => FsError::DirNotFound { path },
			(io::ErrorKind::NotFound, false) => FsError::FileNotFound { path },
			_ => FsError::Io { path, err: e },
		}
	}

	/// Creates an [`FsError::Other`] with a custom error message.
	pub fn other(path: impl AsRef<Path>, err: impl ToString) -> Self {
		FsError::Other {
			path: path.as_ref().into(),
			err: err.to_string(),
		}
	}

	/// Creates an [`FsError::FileNotFound`] for the given path.
	pub fn file_not_found(path: impl AsRef<Path>) -> Self {
		FsError::FileNotFound {
			path: path.as_ref().into(),
		}
	}
}
