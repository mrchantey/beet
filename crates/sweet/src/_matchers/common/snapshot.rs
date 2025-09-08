use super::*;
use crate::prelude::*;
use anyhow::Result;
use beet_utils::prelude::AbsPathBuf;
use beet_utils::prelude::FsExt;
use beet_utils::prelude::ReadFile;
use colorize::AnsiColor;
#[cfg(feature = "tokens")]
use proc_macro2::TokenStream;
#[cfg(feature = "tokens")]
use quote::ToTokens;


// returns whether the assertion should be made
#[allow(dead_code)]
fn parse_snapshot(received: &str) -> Result<Option<String>> {
	let desc = SweetTestCollector::current_test_desc()
		.ok_or_else(|| anyhow::anyhow!("No current test description found"))?;

	// use test name instead of linecol, which would no longer match on any line/col shifts
	let file_name =
		format!(".sweet/snapshots/{}::{}.ron", desc.source_file, desc.name);

	let save_path = AbsPathBuf::new_workspace_rel(file_name)?;

	if std::env::args().any(|arg| arg == "--snap") {
		FsExt::write(&save_path, received)?;
		println!("Snapshot saved: {}", desc.name);
		Ok(None)
	} else {
		let expected = ReadFile::to_string(&save_path).map_err(|_| {

			anyhow::anyhow!(
				"
Snapshot file not found: {}
please run `cargo test -- --snap` to generate, snapshots should be commited to version control

Received:

{}
				",
				&save_path,
				received.to_string().red(),
			)
		})?;
		Ok(Some(expected))
	}
}

// #[cfg(feature = "tokens")]
// impl<T: ToTokens> Matcher<T> {

// }

impl<T> Matcher<T> {
	/// Compares the value to a snapshot, saving it if the `--snap` flag is used.
	/// Snapshots are saved using test name so only one snapshot per test is allowed.
	/// # Panics
	/// If the snapshot file cannot be read or written.
	pub fn to_be_snapshot<M>(&self) -> &Self
	where
		T: StringComp<M>,
	{
		#[cfg(target_arch = "wasm32")]
		{
			beet_utils::log!("snapshot not yet supported on wasm32");
		}
		#[cfg(not(target_arch = "wasm32"))]
		{
			let received = self.value.to_comp_string();
			match parse_snapshot(&received) {
				Ok(Some(expected)) => self.assert_diff(&expected, &received),
				Ok(None) => {
					// snapshot saved, no assertion made
				}
				Err(e) => {
					self.assert(false, &e.to_string());
				}
			}
		}
		self
	}
}

pub trait StringComp<M> {
	fn to_comp_string(&self) -> String;
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> StringComp<Self> for T {
	fn to_comp_string(&self) -> String {
		ron::ser::to_string(&self).expect("Failed to serialize to string")
	}
}

pub struct ToTokensStringCompMarker;

// we dont blanket ToTokens because collision with String
#[cfg(feature = "tokens")]
macro_rules! impl_string_comp_for_tokens {
	($($ty:ty),*) => {
		$(
			impl StringComp<ToTokensStringCompMarker> for $ty {
				fn to_comp_string(&self) -> String {
					pretty_parse(self.to_token_stream())
				}
			}
		)*
	};
}

#[cfg(feature = "tokens")]
impl_string_comp_for_tokens!(
	proc_macro2::TokenStream,
	syn::File,
	syn::Item,
	syn::Expr,
	syn::Stmt,
	syn::Type,
	syn::Pat,
	syn::Ident,
	syn::Block,
	syn::Path,
	syn::Attribute
);

#[cfg(not(feature = "serde"))]
impl<T: ToString> StringComp<Self> for Matcher<T> {
	fn to_comp_string(&self) -> String { self.value.to_string() }
}

/// Attempt to parse the tokens with prettyplease,
/// otherwise return the tokens as a string.
#[cfg(feature = "tokens")]
pub fn pretty_parse(tokens: TokenStream) -> String {
	use syn::File;
	match syn::parse2::<File>(tokens.clone()) {
		Ok(file) => prettyplease::unparse(&file),
		Err(_) => {
			// ok its not a file, lets try again putting the tokens in a function
			match syn::parse2::<File>(quote::quote! {
				fn deleteme(){
						#tokens
				}
			}) {
				Ok(file) => {
					let mut str = prettyplease::unparse(&file);
					str = str.replace("fn deleteme() {\n", "");
					if let Some(pos) = str.rfind("\n}") {
						str.replace_range(pos..pos + 3, "");
					}
					str =
						str.lines()
							.map(|line| {
								if line.len() >= 4 { &line[4..] } else { line }
							})
							.collect::<Vec<_>>()
							.join("\n");
					str
				}
				Err(_) =>
				// ok still cant parse, just return the tokens as a string
				{
					tokens.to_string()
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(serde::Serialize)]
	struct MyStruct(u32);

	#[test]
	fn bool() { MyStruct(7).xpect().to_be_snapshot(); }

	#[cfg(feature = "tokens")]
	#[test]
	fn prettyparse() {
		use quote::quote;
		// valid file
		pretty_parse(quote! {fn main(){let foo = bar;}})
			.xpect()
			.to_be("fn main() {\n    let foo = bar;\n}\n");
		pretty_parse(quote! {let foo = bar; let bazz = boo;})
			.xpect()
			.to_be("let foo = bar;\nlet bazz = boo;");
	}
}
