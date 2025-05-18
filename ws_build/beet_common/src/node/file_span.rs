use crate::prelude::*;
use std::hash::Hash;
use std::path::Path;
use sweet::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Spanner<T> {
	pub value: T,
	/// In the case this was parsed outside of rustc the value span
	/// will be [`Span::call_site()`], so we track the span manually
	span: FileSpan,
}

impl<T> std::ops::Deref for Spanner<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target { &self.value }
}
impl<T> std::ops::DerefMut for Spanner<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<T> Spanner<T> {
	pub fn new(span: FileSpan, value: T) -> Self { Self { value, span } }
	pub fn value(&self) -> &T { &self.value }
	pub fn span(&self) -> &FileSpan { &self.span }
	pub fn into_value(self) -> T { self.value }
	pub fn into_span(self) -> FileSpan { self.span }
}


/// File location of the first symbol inside an rsx macro, used by [RsxTemplate]
/// to reconcile web nodes with templates
/// ## Example
/// ```rust ignore
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileSpan {
	/// Workspace relative path to the file, its essential to use consistent paths
	/// as this struct is created in several places from all kinds concatenations,
	/// and we need PartialEq & Hash to be identical.
	file: WorkspacePathBuf,
	/// The position of the first token in this span
	start: LineCol,
	/// The position of the last token in this span, in cases where the end
	/// is not known this will be the same as start.
	end: LineCol,
}

impl std::fmt::Display for FileSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file.display(), self.start)
	}
}

impl FileSpan {
	#[cfg(feature = "tokens")]
	pub fn new_from_span(
		file: WorkspacePathBuf,
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
			file: WorkspacePathBuf::new(file),
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
			file: WorkspacePathBuf::new(workspace_file_path),
			start,
			end,
		}
	}
	/// Create a new [FileSpan] from a file path, line and column,
	/// most commonly used by the `rsx!` macro.
	/// ## Example
	///
	/// ```rust
	/// # use beet_common::prelude::*;
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
			file: WorkspacePathBuf::new(workspace_file_path),
			start: LineCol::new(line, col),
			end: LineCol::new(line, col),
		}
	}
	pub fn file(&self) -> &WorkspacePathBuf { &self.file }
	pub fn start(&self) -> LineCol { self.start }
	pub fn start_line(&self) -> u32 { self.start.line() }
	pub fn start_col(&self) -> u32 { self.start.col() }
	pub fn end(&self) -> LineCol { self.end }
	pub fn end_line(&self) -> u32 { self.end.line() }
	pub fn end_col(&self) -> u32 { self.end.col() }
}

#[cfg(feature = "tokens")]
impl RustTokens for FileSpan {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let file = self.file.to_string_lossy();
		let start = self.start.into_rust_tokens();
		let end = self.end.into_rust_tokens();
		quote::quote! {
			FileSpan::new(#file, #start, #end)
		}
	}
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[cfg(feature = "tokens")]
	#[test]
	fn to_rust_tokens() {
		let span = FileSpan::new_with_start("foo", 1, 2);
		let tokens = span.into_rust_tokens();
		expect(tokens.to_string()).to_be(
			quote::quote! {
				FileSpan::new("foo", LineCol::new(1, 2), LineCol::new(1, 2))
			}
			.to_string(),
		);
	}
}
