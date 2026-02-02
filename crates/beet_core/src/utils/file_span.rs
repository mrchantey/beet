//! File span utilities for source code location tracking.
//!
//! This module provides types for representing locations within source files,
//! used for error reporting, debugging, and template reconciliation.

use crate::prelude::*;
use std::hash::Hash;
use std::path::Path;


/// Represents a section of a text file.
///
/// When used as a component, this entity represents this section.
/// This is also used to denote the first symbol inside an rsx macro, used by `RsxTemplate`
/// to reconcile web nodes with templates.
///
/// For the generic component version see [`FileSpanOf`].
///
/// # Example
///
/// ```rust,ignore
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FileSpan {
	/// Workspace relative path to the file.
	///
	/// It's essential to use consistent paths as this struct is created in several
	/// places from all kinds of concatenations, and we need [`PartialEq`] and [`Hash`]
	/// to be identical.
	pub file: WsPathBuf,
	/// The position of the first token in this span.
	pub start: LineCol,
	/// The position of the last token in this span.
	///
	/// In cases where the end is not known, this will be the same as start.
	pub end: LineCol,
}

impl std::fmt::Display for FileSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file.display(), self.start)
	}
}

impl FileSpan {
	/// Creates a new [`FileSpan`] with the given file path, start, and end positions.
	pub fn new(
		workspace_file_path: impl AsRef<Path>,
		start: LineCol,
		end: LineCol,
	) -> Self {
		Self {
			file: WsPathBuf::new(workspace_file_path),
			start,
			end,
		}
	}

	/// Creates a new [`FileSpan`] from a panic [`Location`](std::panic::Location).
	pub fn new_from_location(location: &std::panic::Location) -> Self {
		Self {
			file: WsPathBuf::new(location.file()),
			start: LineCol::from_location(location),
			end: LineCol::from_location(location),
		}
	}

	/// Creates a new [`FileSpan`] from a syn spanned item.
	#[cfg(feature = "tokens")]
	pub fn new_from_span(
		file: WsPathBuf,
		spanned: &impl syn::spanned::Spanned,
	) -> Self {
		let span = spanned.span();
		Self {
			file,
			start: span.start().into(),
			end: span.end().into(),
		}
	}

	/// Creates a new [`FileSpan`] representing an entire file.
	///
	/// The line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self {
			file: WsPathBuf::new(file),
			start: LineCol::default(),
			end: LineCol::default(),
		}
	}


	/// Creates a new [`FileSpan`] from a file path, line, and column.
	///
	/// Most commonly used by the `rsx!` macro.
	///
	/// # Example
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// let loc = FileSpan::new_with_start(file!(), line!(), column!());
	/// ```
	///
	/// # Panics
	///
	/// Panics if the line number is 0; lines are 1-indexed.
	pub fn new_with_start(
		workspace_file_path: impl AsRef<Path>,
		line: u32,
		col: u32,
	) -> Self {
		Self {
			file: WsPathBuf::new(workspace_file_path),
			start: LineCol::new(line, col),
			end: LineCol::new(line, col),
		}
	}

	/// Returns the file path.
	pub fn file(&self) -> &WsPathBuf { &self.file }

	/// Returns the start position.
	pub fn start(&self) -> LineCol { self.start }

	/// Returns the start line number.
	pub fn start_line(&self) -> u32 { self.start.line() }

	/// Returns the start column number.
	pub fn start_col(&self) -> u32 { self.start.col() }

	/// Returns the end position.
	pub fn end(&self) -> LineCol { self.end }

	/// Returns the end line number.
	pub fn end_line(&self) -> u32 { self.end.line() }

	/// Returns the end column number.
	pub fn end_col(&self) -> u32 { self.end.col() }
}

/// Trait for types that have a [`FileSpan`].
pub trait GetSpan {
	/// Returns a reference to the span.
	fn span(&self) -> &FileSpan;

	/// Returns a mutable reference to the span.
	fn span_mut(&mut self) -> &mut FileSpan;
}

impl GetSpan for FileSpan {
	fn span(&self) -> &FileSpan { self }
	fn span_mut(&mut self) -> &mut FileSpan { self }
}

/// File span for a specific component type, e.g., `NodeTag` or `AttributeKey`.
///
/// This is a wrapper around [`FileSpan`] that includes a phantom type parameter
/// to associate the span with a specific component type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FileSpanOf<C> {
	/// The underlying file span.
	pub value: FileSpan,
	/// Phantom data for the component type.
	#[reflect(ignore)]
	pub phantom: std::marker::PhantomData<C>,
}


impl<C> std::ops::Deref for FileSpanOf<C> {
	type Target = FileSpan;
	fn deref(&self) -> &Self::Target { &self.value }
}

impl<C> FileSpanOf<C> {
	/// Creates a new [`FileSpanOf`] wrapping the given [`FileSpan`].
	pub fn new(value: FileSpan) -> Self {
		Self {
			value,
			phantom: std::marker::PhantomData,
		}
	}

	/// Consumes self and returns the inner [`FileSpan`].
	pub fn take(self) -> FileSpan { self.value }
}
