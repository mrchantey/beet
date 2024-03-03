use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

#[ext(name=TokenStreamIterExt)]
pub impl<T: Iterator<Item = TokenStream>> T {
	fn collect_comma_punct(self) -> TokenStream {
		let mut out = TokenStream::new();
		for (i, item) in self.enumerate() {
			if i != 0 {
				out.extend(quote! {,});
			}
			out.extend(item);
		}
		out
	}
}

#[ext(name=ResultOptionTokenStreamIterExt)]
pub impl<T: Iterator<Item = Result<Option<TokenStream>>>> T {
	fn collect_tokens(self) -> Result<TokenStream> {
		let out = self
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.filter_map(|x| x)
			.collect_comma_punct();
		Ok(out)
	}
}
