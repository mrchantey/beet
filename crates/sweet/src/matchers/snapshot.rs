use crate::prelude::*;
use anyhow::Result;
use beet_core::prelude::*;
#[cfg(feature = "tokens")]
use proc_macro2::TokenStream;
#[cfg(feature = "tokens")]
use quote::ToTokens;

#[extend::ext(name=SweetSnapshot)]
pub impl<T, M> T
where
	T: StringComp<M>,
{
	/// Compares the value to a snapshot, saving it if the `--snap` flag is used.
	/// Snapshots are saved using test name so only one snapshot per test is allowed.
	/// # Panics
	/// If the snapshot file cannot be read or written.
	#[track_caller]
	fn xpect_snapshot(&self) -> &Self {
		let received = self.to_comp_string();
		match parse_snapshot(&received) {
			Ok(Some(expected)) => {
				panic_ext::assert_diff(&expected, received.into_maybe_not());
			}
			Ok(None) => {
				// snapshot saved, no assertion made
			}
			Err(e) => {
				panic_ext::panic_str(e.to_string());
			}
		}
		self
	}
}


// returns whether the assertion should be made
#[allow(dead_code)]
#[track_caller]
fn parse_snapshot(received: &str) -> Result<Option<String>> {
	let loc = core::panic::Location::caller();
	let snap_name = format!("{}:{}:{}", loc.file(), loc.line(), loc.column());
	// use test name instead of linecol, which would no longer match on any line/col shifts
	let file_name = format!(".sweet/snapshots/{}.ron", snap_name);

	let save_path = AbsPathBuf::new_workspace_rel(file_name)?;

	let env_vars = env_ext::vars();

	if env_vars.iter().any(|arg| arg.0 == "--snap") {
		fs_ext::write(&save_path, received)?;
		beet_core::cross_log!("Snapshot saved: {}", snap_name);
		Ok(None)
	} else {
		let expected = fs_ext::read_to_string(&save_path).map_err(|_| {

			anyhow::anyhow!(
				"
Snapshot file not found: {}
please run `cargo test -- --snap` to generate, snapshots should be commited to version control

Received:

{}
				",
				&save_path,
				paint_ext::red(received),
			)
		})?;

		if env_vars.iter().any(|arg| arg.0 == "--snap-show") {
			beet_core::cross_log!("Snapshot:\n{}", expected);
		}
		Ok(Some(expected))
	}
}

pub trait StringComp<M> {
	fn to_comp_string(&self) -> String;
}

// #[cfg(feature = "serde")]
// impl<T: 'static + serde::Serialize> StringComp<Self> for T {
// 	fn to_comp_string(&self) -> String {
// 		use std::any::TypeId;
// 		let ty_str = TypeId::of::<str>();

// 		match TypeId::of::<T>() {
// 			ty_str => s.to_string(),
// 			other => ron::ser::to_string(&self)
// 				.expect("Failed to serialize to string"),
// 		}
// 	}
// }

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

// #[cfg(not(feature = "serde"))]
macro_rules! impl_string_comp_for_primitives {
	($($ty:ty),*) => {
		$(
			impl StringComp<$ty> for $ty {
				fn to_comp_string(&self) -> String {
					self.to_string()
				}
			}
		)*
	};
}

impl_string_comp_for_primitives!(
	&str,
	String,
	std::borrow::Cow<'static, str>,
	bool
);

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


	#[test]
	fn bool() { true.xpect_snapshot(); }

	// #[test]
	// fn serde_struct() {
	// 	#[derive(serde::Serialize)]
	// 	struct MyStruct(u32);

	// 	MyStruct(7).xpect_snapshot();
	// }

	#[cfg(feature = "tokens")]
	#[test]
	fn prettyparse() {
		use quote::quote;
		// valid file
		pretty_parse(quote! {fn main(){let foo = bar;}})
			.xpect_eq("fn main() {\n    let foo = bar;\n}\n");
		pretty_parse(quote! {let foo = bar; let bazz = boo;})
			.xpect_eq("let foo = bar;\nlet bazz = boo;");
	}
}
