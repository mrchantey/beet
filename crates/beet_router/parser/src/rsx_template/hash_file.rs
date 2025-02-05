use anyhow::Result;
use beet_rsx::prelude::RstmlRustToHash;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use sweet::prelude::ReadFile;


/// determine if a compilation is required due to rust code changes
#[derive(Default)]
pub struct HashRsxFile;


impl HashRsxFile {
	/// # Errors
	/// If the file cannot be read or parsed as tokens
	pub fn hash_file(path: &Path) -> Result<u64> {
		let file = syn::parse_file(&ReadFile::to_string(path)?).unwrap();
		Ok(hash(file.to_token_stream()))
	}
	/// # Errors
	/// If the string cannot be parsed as tokens
	pub fn hash_string(string: &str) -> Result<u64> {
		let file = syn::parse_file(&string)?;
		Ok(hash(file.to_token_stream()))
	}
	pub fn hash_tokens(tokens: TokenStream) -> u64 { hash(tokens) }
}

fn hash(tokens: TokenStream) -> u64 {
	let mut hasher = DefaultHasher::new();
	let mut iter = tokens.into_iter().peekable();

	while let Some(token) = iter.next() {
		match &token {
			// hash rsx!{...}
			TokenTree::Ident(ident) if ident.to_string() == "rsx" => {
				if let Some(TokenTree::Punct(punct)) = iter.peek() {
					if punct.as_char() == '!' {
						iter.next(); // consume !
						if let Some(TokenTree::Group(group)) = iter.next() {
							RstmlRustToHash::hash(&mut hasher, group.stream());
							continue;
						}
					}
				}
			}
			_ => {
				// Hash everything else
				token.to_string().hash(&mut hasher);
			}
		}
	}

	hasher.finish()
}


#[cfg(test)]
mod test {
	use super::*;
	use quote::quote;
	use sweet::prelude::*;
	#[test]
	fn works() {
		// ignore element names
		expect(hash(quote! {rsx!{<el1/>}})).to_be(hash(quote! {rsx!{<el2>}}));
		// ignore literals
		expect(hash(quote! {rsx!{<el key="lit"/>}}))
			.to_be(hash(quote! {rsx!{<el key=28/>}}));
		// ignore attr keys
		expect(hash(quote! {rsx!{<el foo={7}/>}}))
			.to_be(hash(quote! {rsx!{<el bar={7}>}}));
		// ignore html location - block
		expect(hash(quote! {rsx!{<el>{7}</el>}}))
			.to_be(hash(quote! {rsx!{<el><el>{7}</el></el>}}));
		// ignore html location - component
		expect(hash(quote! {rsx!{<el><Component></el>}}))
			.to_be(hash(quote! {rsx!{<el><el><Component></el></el>}}));
		// hash external pre
		expect(hash(quote! {let foo = rsx!{<el/>}}))
			.not()
			.to_be(hash(quote! {let bar = rsx!{<el/>}}));
		// hash external post
		expect(hash(quote! {rsx!{<el/>}foo}))
			.not()
			.to_be(hash(quote! {rsx!{<el/>}bar}));
		// hash order
		expect(hash(quote! {rsx!{<el>{7}{8}</el>}}))
			.not()
			.to_be(hash(quote! {rsx!{<el>{8}{7}</el>}}));
		// hash attr idents
		expect(hash(quote! {rsx!{<el foo=bar/>}}))
			.not()
			.to_be(hash(quote! {rsx!{<el foo=bazz>}}));
		// hash attr blocks
		expect(hash(quote! {rsx!{<el {7}/>}}))
			.not()
			.to_be(hash(quote! {rsx!{<el {8}>}}));
		// hash node blocks
		expect(hash(quote! {rsx!{<el>{7}</el>}}))
			.not()
			.to_be(hash(quote! {rsx!{<el>{8}</el>}}));
	}
	
}

