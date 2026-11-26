//! [`SourceSpan`] and [`Spanned`]: where a piece of markup was written.
use crate::prelude::*;
use core::ops::Range;

/// The byte range a piece of markup was written at in its document, unknown
/// for one a directive wrote.
///
/// Never part of what markup says, so two spans are always equal: a tree
/// equals another that says the same wherever either was written.
#[derive(Debug, Default, Clone)]
pub struct SourceSpan(Option<Range<u32>>);

impl PartialEq for SourceSpan {
	fn eq(&self, _other: &Self) -> bool { true }
}

impl SourceSpan {
	/// The range `start..end`, in bytes.
	pub fn new(start: usize, end: usize) -> Self {
		Self(Some(start as u32..end as u32))
	}

	/// The range, if it is known.
	pub fn get(&self) -> Option<&Range<u32>> { self.0.as_ref() }

	/// Whether the range is known.
	pub fn is_known(&self) -> bool { self.0.is_some() }

	/// The range reaching from this one's start to `other`'s end, or whichever
	/// of the two is known.
	pub fn to(&self, other: &Self) -> Self {
		match (&self.0, &other.0) {
			(Some(start), Some(end)) => Self(Some(start.start..end.end)),
			(Some(span), None) | (None, Some(span)) => Self(Some(span.clone())),
			(None, None) => Self(None),
		}
	}

	/// `error` at the start of this range, where it is known.
	pub fn locate(&self, error: BevyError) -> BevyError {
		match &self.0 {
			Some(span) => SourceError::at(error, span.start as usize),
			None => error,
		}
	}
}

impl From<Range<usize>> for SourceSpan {
	fn from(range: Range<usize>) -> Self { Self::new(range.start, range.end) }
}

/// A value and where it was written.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
pub struct Spanned<T> {
	/// What was written.
	#[deref]
	pub value: T,
	/// Where.
	pub span: SourceSpan,
}

impl<T> Spanned<T> {
	/// `value`, written at `span`.
	pub fn new(value: T, span: impl Into<SourceSpan>) -> Self {
		Self {
			value,
			span: span.into(),
		}
	}
}

impl<T: core::fmt::Display> core::fmt::Display for Spanned<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.value.fmt(f)
	}
}

/// A value written nowhere in particular.
impl<T> From<T> for Spanned<T> {
	fn from(value: T) -> Self { Self::new(value, SourceSpan::default()) }
}

impl From<&str> for Spanned<String> {
	fn from(value: &str) -> Self { value.to_string().into() }
}
