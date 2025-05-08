use std::hash::Hash;
use std::path::Path;
use sweet::prelude::*;


/// File location of the first symbol inside an rsx macro, used by [RsxTemplate]
/// to reconcile web nodes with templates
///
/// ```rust ignore
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeSpan {
	/// Workspace relative path to the file, its essential to use consistent paths
	/// as this struct is created in several places from all kinds concatenations,
	/// and we need PartialEq & Hash to be identical.
	pub file: WorkspacePathBuf,
	/// The start position of the first token in this span
	pub start: LineCol,
}

/// A location in a source file, the line is 1 indexed and the column is 0 indexed.
/// The Default implementation is `1:0`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineCol {
	/// The 1 indexed line in the source file, reflecting the behavior of `line!()` and
	/// `proc_macro2::Span`
	pub line: u32,
	/// The 0 indexed column in the source file, reflecting the behavior of `column!()`
	/// and `proc_macro2::Span`. This is not the same as proc_macro::Span which
	/// is 1 indexed.
	pub col: u32,
}

impl LineCol {
	pub fn new(line: u32, col: u32) -> Self {
		// id like to assert this but it seems rust-analyzer uses 0 based line numbers?

		// assert_ne!(line, 0, "Line number must be greater than 0");
		Self { line, col }
	}

	#[cfg(feature = "tokens")]
	pub fn new_from_span_start(span: &impl syn::spanned::Spanned) -> Self {
		let span = span.span();
		Self {
			line: span.start().line as u32,
			col: span.start().column as u32,
		}
	}
}

impl Default for LineCol {
	fn default() -> Self { Self { line: 1, col: 0 } }
}

impl std::fmt::Display for LineCol {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.line, self.col)
	}
}

#[cfg(feature = "tokens")]
impl crate::prelude::SerdeTokens for LineCol {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let line = proc_macro2::Literal::u32_unsuffixed(self.line);
		let col = proc_macro2::Literal::u32_unsuffixed(self.col);
		quote::quote! {
			LineCol::new(#line, #col)
		}
	}

	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		let line = proc_macro2::Literal::u32_unsuffixed(self.line);
		let col = proc_macro2::Literal::u32_unsuffixed(self.col);
		quote::quote! {
			LineCol(
				line: #line,
				col: #col
			)
		}
	}
}


impl std::fmt::Display for NodeSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.file.display(), self.start)
	}
}

impl NodeSpan {
	pub fn new_from_file(file: WorkspacePathBuf) -> Self {
		Self {
			file,
			start: LineCol::default(),
		}
	}

	#[cfg(feature = "tokens")]
	pub fn new_from_span_start(
		file: WorkspacePathBuf,
		span: &impl syn::spanned::Spanned,
	) -> Self {
		Self {
			file,
			start: LineCol::new_from_span_start(span),
		}
	}


	pub fn placeholder() -> Self {
		Self {
			file: WorkspacePathBuf::default(),
			start: LineCol::default(),
		}
	}
	/// Create a new [NodeSpan] from a file path where it should represent
	/// the entire file, the line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self {
			file: WorkspacePathBuf::new(file),
			start: LineCol::default(),
		}
	}

	/// Create a new [NodeSpan] from a file path, line and column,
	/// most commonly used by the `rsx!` macro.
	/// ## Example
	///
	/// ```rust
	/// # use beet_common::prelude::*;
	/// let loc = NodeSpan::new(file!(), line!(), column!());
	/// ```
	/// ## Panics
	/// Panics if the line number is 0, lines are 1 indexed.
	pub fn new(
		workspace_file_path: impl AsRef<Path>,
		line: u32,
		col: u32,
	) -> Self {
		Self {
			file: WorkspacePathBuf::new(workspace_file_path),
			start: LineCol::new(line, col),
		}
	}
	pub fn new_with_start(
		workspace_file_path: impl AsRef<Path>,
		start: LineCol,
	) -> Self {
		Self {
			file: WorkspacePathBuf::new(workspace_file_path),
			start,
		}
	}
	pub fn file(&self) -> &WorkspacePathBuf { &self.file }
	pub fn start_line(&self) -> u32 { self.start.line }
	pub fn start_col(&self) -> u32 { self.start.col }
}

#[cfg(feature = "tokens")]
impl crate::prelude::SerdeTokens for NodeSpan {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let file = self.file.to_string_lossy();
		let line = proc_macro2::Literal::u32_unsuffixed(self.start.line);
		let col = proc_macro2::Literal::u32_unsuffixed(self.start.col);
		quote::quote! {
			NodeSpan::new(#file, #line, #col)
		}
	}

	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		let file = self.file.to_string_lossy();
		let linecol = self.start.into_ron_tokens();
		quote::quote! {
			NodeSpan(
				file: (#file),
				start: #linecol,
			)
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;


	#[cfg(feature = "tokens")]
	#[test]
	fn to_rust_tokens() {
		let span = NodeSpan::new("foo", 1, 2);
		let tokens = span.into_rust_tokens();
		expect(tokens.to_string()).to_be(
			quote::quote! {
				NodeSpan::new("foo",1, 2)
			}
			.to_string(),
		);
	}
	#[cfg(feature = "tokens")]
	#[test]
	fn to_ron_tokens() {
		let span = NodeSpan::new("foo", 1, 2);
		let tokens = span.into_ron_tokens();
		let span2 = ron::de::from_str::<NodeSpan>(&tokens.to_string()).unwrap();
		expect(span2).to_be(span);
	}
}
