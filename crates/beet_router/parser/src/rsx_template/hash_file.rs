use anyhow::Result;
use beet_rsx::prelude::RstmlRustToHash;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use sweet::prelude::ReadFile;


/// determine if a compilation is required due to rust code changes
/// by hashing an entire file
pub struct HashRsxFile {
	hasher: RapidHasher,
}

impl Default for HashRsxFile {
	fn default() -> Self {
		Self {
			hasher: RapidHasher::default_const(),
		}
	}
}

impl HashRsxFile {
	pub fn file_to_hash(path: &Path) -> Result<u64> {
		let file = ReadFile::to_string(path)?;
		let mut this = Self::default();
		this.walk_tokens(file.to_token_stream())?;
		Ok(this.hasher.finish())
	}

	/// # Errors
	/// If the file cannot be read or parsed as tokens
	pub fn walk_tokens(&mut self, tokens: TokenStream) -> Result<()> {
		let mut iter = tokens.into_iter().peekable();
		while let Some(tree) = iter.next() {
			match &tree {
				TokenTree::Ident(ident) if ident.to_string() == "rsx" => {
					if let Some(TokenTree::Punct(punct)) = iter.peek() {
						if punct.as_char() == '!' {
							iter.next(); // consume !
							if let Some(TokenTree::Group(group)) = iter.next() {
								RstmlRustToHash::visit_and_hash(
									&mut self.hasher,
									group.stream(),
								);
								continue;
							}
						}
					}
				}
				TokenTree::Group(group) => {
					// recurse into groups
					self.walk_tokens(group.stream())?;
				}
				tree => {
					// Hash everything else
					tree.to_string().replace(" ", "").hash(&mut self.hasher);
				}
			}
		}
		Ok(())
	}
}




#[cfg(test)]
mod test {
	use super::*;
	use quote::quote;
	use sweet::prelude::*;
	#[test]
	// #[ignore = "todo visitor pattern for nesting"]
	#[rustfmt::skip]
	fn works() {		
		fn hash(tokens: TokenStream) -> u64 {
			let mut hasher = HashRsxFile::default();
			hasher.walk_tokens(tokens).unwrap();
			hasher.hasher.finish()
		}

		// ignore element names
		expect(hash(quote! {rsx!{<el1/>}}))
		.to_be(hash(quote! {rsx!{<el2/>}}));
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
		
		// visits nested
		expect(hash(quote! {{v1}}))
    .not()
		.to_be(hash(quote! {{v2}}));
		expect(hash(quote! {{rsx!{<el1/>}}}))
		.to_be(hash(quote! {{rsx!{<el2/>}}}));
	}
}
