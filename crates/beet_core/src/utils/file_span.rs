//! File span utilities for source code location tracking.
//!
//! This module provides types for representing locations within source files,
//! used for error reporting, debugging, and template reconciliation.
//!
//! The rich std [`FileSpan`] is backed by a [`WsPathBuf`] (filesystem paths,
//! workspace-relative resolution). The bare-metal test build (no_std) compiles
//! a minimal [`FileSpan`] backed by a [`SmolStr`] instead: it is only a field in
//! the test-outcome types there (a panic location is never constructed on
//! device under the abort model), so it needs no path machinery.

#[allow(unused_imports)]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::hash::Hash;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::sync::Arc;


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
#[cfg(feature = "std")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FileSpan {
	/// Workspace relative path to the file.
	///
	/// It's essential to use consistent paths as this struct is created in several
	/// places from all kinds of concatenations, and we need [`PartialEq`] and [`Hash`]
	/// to be identical.
	/// This field is [`Arc`] as FileSpan is frequently cloned.
	path: Arc<WsPathBuf>,
	/// The position of the first token in this span.
	start: LineCol,
	/// The position of the last token in this span.
	///
	/// In cases where the end is not known, this will be the same as start.
	end: LineCol,
}

#[cfg(feature = "std")]
impl std::fmt::Display for FileSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.path.display(), self.start)
	}
}

#[cfg(feature = "std")]
impl FileSpan {
	/// Creates a new [`FileSpan`] with the given file path, start, and end positions.
	pub fn new(
		workspace_file_path: impl AsRef<Path>,
		start: LineCol,
		end: LineCol,
	) -> Self {
		Self {
			path: Arc::new(WsPathBuf::new(workspace_file_path)),
			start,
			end,
		}
	}

	/// Create a new [`FileSpan`] from the caller's location.
	/// Be sure to add `#[track_caller]` to your method if you want to propagate
	/// an outer caller.
	#[track_caller]
	pub fn new_caller() -> Self {
		Self::new_from_location(std::panic::Location::caller())
	}

	/// Creates a new [`FileSpan`] from a panic [`Location`](std::panic::Location).
	pub fn new_from_location(location: &std::panic::Location) -> Self {
		Self::new(
			WsPathBuf::new(location.file()),
			LineCol::from_location(location),
			LineCol::from_location(location),
		)
	}

	/// Creates a new [`FileSpan`] from a syn spanned item.
	#[cfg(feature = "tokens")]
	pub fn new_from_span(
		file: WsPathBuf,
		spanned: &impl syn::spanned::Spanned,
	) -> Self {
		let span = spanned.span();
		Self::new(file, span.start().into(), span.end().into())
	}


	/// Creates a new [`FileSpan`] representing an entire file.
	///
	/// The line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self::new(WsPathBuf::new(file), LineCol::default(), LineCol::default())
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
		Self::new(
			WsPathBuf::new(workspace_file_path),
			LineCol::new(line, col),
			LineCol::new(line, col),
		)
	}

	/// Returns the file path.
	pub fn path(&self) -> &WsPathBuf { &self.path }

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

/// Minimal no_std [`FileSpan`]: a `SmolStr` path plus start/end positions.
///
/// On bare metal this is only ever a field in the test-outcome types; a panic
/// location is never built (the abort model never reaches the catch path), so
/// it needs none of the std path resolution.
#[cfg(not(feature = "std"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSpan {
	path: SmolStr,
	start: LineCol,
	end: LineCol,
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for FileSpan {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}:{}", self.path, self.start)
	}
}

#[cfg(not(feature = "std"))]
impl FileSpan {
	/// Creates a new [`FileSpan`] from a workspace-relative file path.
	pub fn new(
		workspace_file_path: impl Into<SmolStr>,
		start: LineCol,
		end: LineCol,
	) -> Self {
		Self {
			path: workspace_file_path.into(),
			start,
			end,
		}
	}

	/// Creates a new [`FileSpan`] from a file path, line, and column.
	pub fn new_with_start(
		workspace_file_path: impl Into<SmolStr>,
		line: u32,
		col: u32,
	) -> Self {
		Self::new(workspace_file_path, LineCol::new(line, col), LineCol::new(line, col))
	}

	/// Returns the file path.
	pub fn path(&self) -> &str { &self.path }

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
#[cfg(feature = "std")]
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


#[cfg(feature = "std")]
impl<C> std::ops::Deref for FileSpanOf<C> {
	type Target = FileSpan;
	fn deref(&self) -> &Self::Target { &self.value }
}

#[cfg(feature = "std")]
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
