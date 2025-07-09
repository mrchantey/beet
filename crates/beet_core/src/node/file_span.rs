use crate::as_beet::*;
use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::path::Path;

/// File location of the first symbol inside an rsx macro, used by [RsxTemplate]
/// to reconcile web nodes with templates.
/// For the component version see [`FileSpanOf`].
/// ## Example
/// ```rust ignore
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FileSpan {
	/// Workspace relative path to the file, its essential to use consistent paths
	/// as this struct is created in several places from all kinds concatenations,
	/// and we need PartialEq & Hash to be identical.
	pub file: WsPathBuf,
	/// The position of the first token in this span
	pub start: LineCol,
	/// The position of the last token in this span, in cases where the end
	/// is not known this will be the same as start.
	pub end: LineCol,
}

impl std::fmt::Display for FileSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file.display(), self.start)
	}
}

impl FileSpan {
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

	/// Create a new [FileSpan] from a file path where it should represent
	/// the entire file, the line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self {
			file: WsPathBuf::new(file),
			start: LineCol::default(),
			end: LineCol::default(),
		}
	}

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
	/// Create a new [FileSpan] from a file path, line and column,
	/// most commonly used by the `rsx!` macro.
	/// ## Example
	///
	/// ```rust
	/// # use beet_core::prelude::*;
	/// let loc = FileSpan::new_with_start(file!(), line!(), column!());
	/// ```
	/// ## Panics
	/// Panics if the line number is 0, lines are 1 indexed.
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
	pub fn file(&self) -> &WsPathBuf { &self.file }
	pub fn start(&self) -> LineCol { self.start }
	pub fn start_line(&self) -> u32 { self.start.line() }
	pub fn start_col(&self) -> u32 { self.start.col() }
	pub fn end(&self) -> LineCol { self.end }
	pub fn end_line(&self) -> u32 { self.end.line() }
	pub fn end_col(&self) -> u32 { self.end.col() }
}

pub trait GetSpan {
	fn span(&self) -> &FileSpan;
	// probs an anti-pattern but need it until proper spans in rsx combinator
	fn span_mut(&mut self) -> &mut FileSpan;
}

impl GetSpan for FileSpan {
	fn span(&self) -> &FileSpan { self }
	fn span_mut(&mut self) -> &mut FileSpan { self }
}

/// File span for a specific component type, ie [`NodeTag`] or [`AttributeKey`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct FileSpanOf<C> {
	pub value: FileSpan,
	#[reflect(ignore)]
	pub phantom: std::marker::PhantomData<C>,
}


impl<C> std::ops::Deref for FileSpanOf<C> {
	type Target = FileSpan;
	fn deref(&self) -> &Self::Target { &self.value }
}

impl<C> FileSpanOf<C> {
	pub fn new(value: FileSpan) -> Self {
		Self {
			value,
			phantom: std::marker::PhantomData,
		}
	}
	pub fn take(self) -> FileSpan { self.value }
}
#[cfg(feature = "tokens")]
#[derive(Debug, Clone, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct SpanOf<C> {
	pub value: send_wrapper::SendWrapper<proc_macro2::Span>,
	pub phantom: std::marker::PhantomData<C>,
}


#[cfg(feature = "tokens")]
impl<C> std::ops::Deref for SpanOf<C> {
	type Target = proc_macro2::Span;
	fn deref(&self) -> &Self::Target { &self.value }
}

#[cfg(feature = "tokens")]
impl<C> SpanOf<C> {
	pub fn new(value: proc_macro2::Span) -> Self {
		Self {
			value: send_wrapper::SendWrapper::new(value),
			phantom: std::marker::PhantomData,
		}
	}
	pub fn take(self) -> proc_macro2::Span { self.value.take() }
}
