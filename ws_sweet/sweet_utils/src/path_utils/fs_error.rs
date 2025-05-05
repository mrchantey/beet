use std::io;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

pub type FsResult<T> = std::result::Result<T, FsError>;


/// An fs error that actuall outputs the missing path
#[derive(Debug, Error)]
pub enum FsError {
	#[error("Fs Error - File Not Found\nPath: {path}")]
	FileNotFound { path: PathBuf },
	#[error("Fs Error - Dir Not Found\nPath: {path}")]
	DirNotFound { path: PathBuf },
	/// Catch-all error for io errors that are not [`io::ErrorKind::NotFound`]
	#[error("Fs Error - IO\nPath: {path}\nError: {err}")]
	Io { path: PathBuf, err: io::Error },
	/// fs::read_dir may succeed but reading one child may fail
	/// in which case we dont know the child path
	#[error("Fs Error - Child IO\nParent: {parent}\nError: {err}")]
	ChildIo { parent: PathBuf, err: io::Error },
	#[error("Fs Error\nPath: {path}\nError: {err}")]
	Other { path: PathBuf, err: String },
}

impl FsError {
	pub fn assert_dir(path: impl AsRef<Path>) -> FsResult<()> {
		if !path.as_ref().is_dir() {
			Err(FsError::DirNotFound {
				path: path.as_ref().into(),
			})
		} else {
			Ok(())
		}
	}


	pub fn io(path: impl AsRef<Path>, e: io::Error) -> Self {
		let path: PathBuf = path.as_ref().into();
		match (e.kind(), path.is_dir()) {
			(io::ErrorKind::NotFound, true) => FsError::DirNotFound { path },
			(io::ErrorKind::NotFound, false) => FsError::FileNotFound { path },
			_ => FsError::Io { path, err: e },
		}
	}

	pub fn other(path: impl AsRef<Path>, err: impl ToString) -> Self {
		FsError::Other {
			path: path.as_ref().into(),
			err: err.to_string(),
		}
	}

	pub fn file_not_found(path: impl AsRef<Path>) -> Self {
		FsError::FileNotFound {
			path: path.as_ref().into(),
		}
	}
}
