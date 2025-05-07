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
	/// The 1 indexed line in the source file, reflecting the behavior of `line!()`
	pub line: u32,
	/// The 0 indexed column in the source file, reflecting the behavior of `column!()`
	pub col: u32,
}

impl std::fmt::Display for NodeSpan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file.display(), self.line, self.col)
	}
}

impl NodeSpan {
	pub fn new_from_file(file: WorkspacePathBuf) -> Self {
		Self {
			file,
			line: 1,
			col: 0,
		}
	}

	#[cfg(feature = "tokens")]
	pub fn new_from_spanned(
		file: WorkspacePathBuf,
		spanned: &impl syn::spanned::Spanned,
	) -> Self {
		let span = spanned.span();
		Self {
			file,
			line: span.start().line as u32,
			col: span.start().column as u32,
		}
	}


	pub fn placeholder() -> Self {
		Self {
			file: WorkspacePathBuf::default(),
			line: 1,
			col: 0,
		}
	}
	/// Create a new [NodeSpan] from a file path where it should represent
	/// the entire file, the line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self {
			file: WorkspacePathBuf::new(file),
			line: 1,
			col: 0,
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
		// id like to assert this but it seems rust-analyzer uses 0 based line numbers?
		// assert_ne!(line, 0, "Line number must be greater than 0");
		Self {
			file: WorkspacePathBuf::new(workspace_file_path),
			line,
			col,
		}
	}
	pub fn file(&self) -> &WorkspacePathBuf { &self.file }
	pub fn line(&self) -> u32 { self.line }
	pub fn col(&self) -> u32 { self.col }
}

#[cfg(feature = "tokens")]
impl crate::prelude::SerdeTokens for NodeSpan {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let file = self.file.to_string_lossy();
		let line = self.line;
		let col = self.col;
		quote::quote! {
			NodeSpan::new(#file, #line, #col)
		}
	}

	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		let file = self.file.to_string_lossy();
		let line = proc_macro2::Literal::u32_unsuffixed(self.line);
		let col = proc_macro2::Literal::u32_unsuffixed(self.col);
		quote::quote! {
			NodeSpan(
				file: (#file),
				line: #line,
				col: #col)
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
				NodeSpan::new("foo", 1u32, 2u32)
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
