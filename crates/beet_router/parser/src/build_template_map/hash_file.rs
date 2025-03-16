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
/// by hashing an entire file.
/// Non-rust files can also be handled
#[derive(Default)]
pub struct HashFile {
	/// whether non-rust files should also be hashed
	hash_non_rust: bool,
}

impl HashFile {
	/// hash only the code parts of a rust file.
	pub fn file_to_hash(&self, path: &Path) -> Result<u64> {
		let file = ReadFile::to_string(path)?;
		if path.extension().map_or(false, |ext| ext == "rs") {
			let mut hasher = RapidHasher::default_const();
			// parse to file or it will be a string literal
			let file = syn::parse_file(&file)?;
			self.walk_tokens(&mut hasher, file.to_token_stream())?;
			Ok(hasher.finish())
		} else if self.hash_non_rust {
			let mut hasher = RapidHasher::default_const();
			hasher.write(file.as_bytes());
			Ok(hasher.finish())
			// other files will not trigger a recompile by default
		} else {
			return Ok(0);
		}
	}

	/// # Errors
	/// If the file cannot be read or parsed as tokens
	fn walk_tokens(
		&self,
		hasher: &mut RapidHasher,
		tokens: TokenStream,
	) -> Result<()> {
		let mut iter = tokens.into_iter().peekable();
		while let Some(tree) = iter.next() {
			// println!("visiting tree: {}", tree.to_string());
			match &tree {
				TokenTree::Ident(ident) if ident.to_string() == "rsx" => {
					// println!("visiting ident: {}", ident.to_string());
					if let Some(TokenTree::Punct(punct)) = iter.peek() {
						if punct.as_char() == '!' {
							iter.next(); // consume !
							if let Some(TokenTree::Group(group)) = iter.next() {
								// println!("visiting rsx");
								RstmlRustToHash::visit_and_hash(
									hasher,
									group.stream(),
								);
								continue;
							}
						}
					} else {
						ident.to_string().replace(" ", "").hash(hasher);
					}
				}
				TokenTree::Group(group) => {
					// recurse into groups
					self.walk_tokens(hasher, group.stream())?;
				}
				tree => {
					// Hash everything else
					tree.to_string().replace(" ", "").hash(hasher);
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
			let mut hasher = RapidHasher::default_const();
			HashFile::default().walk_tokens(&mut hasher,tokens).unwrap();
			hasher.finish()
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
