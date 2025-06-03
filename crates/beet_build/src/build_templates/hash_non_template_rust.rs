use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::ReadFile;

pub(super) struct HashNonTemplateRust<'a, H> {
	pub macros: &'a TemplateMacros,
	pub hasher: &'a mut H,
}
impl<H: Hasher> HashNonTemplateRust<'_, H> {
	pub fn hash(&mut self, file: &TemplateFile) -> Result<()> {
		let path = file.path();
		match path.extension() {
			Some(ex) if ex == "rs" => {
				let file_content = ReadFile::to_string(path.into_abs())?;
				let parsed_file =
					syn::parse_file(&file_content).map_err(|err| {
						anyhow::anyhow!(
							"Failed to parse file: {}\n{}",
							path.display(),
							err
						)
					})?;
				self.walk_tokens(parsed_file.to_token_stream())?;
				Ok(())
			}
			_ => {
				// currently all non-rust files (mdx,rsx) are themselves a parsed expression
				// so the hash is built from that expression
				Ok(())
			}
		}
	}
	/// # Errors
	/// If the file cannot be read or parsed as tokens
	fn walk_tokens(&mut self, tokens: TokenStream) -> Result<()> {
		let mut iter = tokens.into_iter().peekable();
		while let Some(tree) = iter.next() {
			// println!("visiting tree: {}", tree.to_string());
			match &tree {
				TokenTree::Ident(ident)
					if ident.to_string() == self.macros.rstml =>
				{
					// println!("visiting ident: {}", ident.to_string());
					if let Some(TokenTree::Punct(punct)) = iter.peek() {
						if punct.as_char() == '!' {
							iter.next(); // consume !
							if let Some(TokenTree::Group(_group)) = iter.next()
							{
								// inside the template, this will be hashed by
								// update_file_expr_hash
								continue;
							}
						}
					} else {
						ident.to_string().hash(self.hasher);
						// i dont think we need to replace spaces here, thats for
						// interop between tokenizers but we're always using syn::parse_file
						// ident.to_string().replace(" ", "").hash(self.hasher);
					}
				}
				TokenTree::Group(group) => {
					// recurse into groups
					self.walk_tokens(group.stream())?;
				}
				tree => {
					// Hash everything else
					tree.to_string().hash(self.hasher);
					// i dont think we need to replace spaces here, thats for
					// interop between tokenizers but we're always using syn::parse_file
					// tree.to_string().replace(" ", "").hash(self.hasher);
				}
			}
		}
		Ok(())
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use rapidhash::RapidHasher;
	use std::hash::Hasher;
	use sweet::prelude::*;

	fn hash(tokens: TokenStream) -> u64 {
		let mut hasher = RapidHasher::default_const();
		HashNonTemplateRust {
			hasher: &mut hasher,
			macros: &TemplateMacros::default(),
		}
		.walk_tokens(tokens)
		.unwrap();
		hasher.finish()
	}


	#[test]
	#[rustfmt::skip]
	fn works() {
		// ignore macro inners
		expect(hash(quote! {rsx!{1}}))
		.to_be(hash(quote! {rsx!{2}}));
		// ignore multiple macros
		expect(hash(quote! {rsx!{1} rsx!{1}}))
		.to_be(hash(quote! {rsx!{2} rsx!{2}}));
		// hash non-template expressions
		expect(hash(quote! {foo}))
		.not()
		.to_be(hash(quote! {bar}));
		// hash other macros
		expect(hash(quote! {println!(foo)}))
		.not()
		.to_be(hash(quote! {println!(bar)}));
	}
}
