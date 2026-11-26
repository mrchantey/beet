//! [`SourceError`], an error at a byte offset of the document being read.
use crate::prelude::*;
use thiserror::Error;

/// An error at a byte offset of the document being read, which the reader
/// holding its source shows as a [`SourcePosition`].
#[derive(Clone, Error)]
#[error("{error}")]
pub struct SourceError {
	offset: u32,
	error: CloneError,
}

/// As its inner error reads, the offset meaning nothing without its source.
impl core::fmt::Debug for SourceError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Debug::fmt(&self.error, f)
	}
}

impl SourceError {
	/// `error` at `offset`, unless it already knows where it is: the innermost
	/// place is the nearest.
	pub fn at(error: BevyError, offset: usize) -> BevyError {
		match error.downcast_ref::<Self>() {
			Some(_) => error,
			None => Self {
				offset: offset as u32,
				error: CloneError::new(error),
			}
			.into(),
		}
	}

	/// `error` with its place counted from `offset` further back, for one read
	/// from a slice of its document, else at `offset` itself.
	pub fn within(error: BevyError, offset: usize) -> BevyError {
		match error.downcast_ref::<Self>().cloned() {
			Some(inner) => Self {
				offset: inner.offset + offset as u32,
				error: inner.error,
			}
			.into(),
			None => Self::at(error, offset),
		}
	}

	/// The place `error` knows it is at, if it does, and the error itself.
	pub fn split(error: BevyError) -> (Option<u32>, BevyError) {
		match error.downcast_ref::<Self>().cloned() {
			Some(inner) => (Some(inner.offset), inner.error.into()),
			None => (None, error),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The innermost place wins, and a slice's counts from its own start.
	#[crate::test]
	fn the_innermost_place_wins() {
		let error = SourceError::at(bevyhow!("nope"), 4);
		let error = SourceError::at(error, 9);
		let (offset, error) =
			SourceError::split(SourceError::within(error, 10));
		offset.xpect_eq(Some(14));
		error.to_string().trim().xpect_contains("nope");
		SourceError::split(bevyhow!("nope")).0.xpect_none();
	}
}
