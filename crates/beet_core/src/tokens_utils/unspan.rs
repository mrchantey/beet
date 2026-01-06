use crate::prelude::*;
use quote::ToTokens;
use std::fmt::Display;
use syn::parse::Parse;




/// Wrapper for [`ToTokens`] types that parses the stream, removing span
/// information and allowing the value to be sent across threads.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref)]
pub struct Unspan<T>(T);


impl<T: ToTokens + Parse> Unspan<T> {
	pub fn new(value: &T) -> Self {
		let value = value.to_token_stream().to_string();
		let value =
			syn::parse_str(&value).expect("Failed to parse token stream");
		Self(value)
	}
	pub fn parse_str(value: &str) -> syn::Result<Self> {
		let value = syn::parse_str(value)?;
		Ok(Self(value))
	}
}

impl<T: ToTokens + Parse> From<T> for Unspan<T> {
	fn from(value: T) -> Self { Self::new(&value) }
}

impl<T: ToTokens> ToTokens for Unspan<T> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		self.0.to_tokens(tokens);
	}
}


impl<T: Display> Display for Unspan<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

// SAFETY: The span information has been removed from the inner value,
// so Unspan<T> is safe to send and share across threads.
unsafe impl<T> Send for Unspan<T> {}
// SAFETY: The span information has been removed from the inner value,
// so Unspan<T> is safe to send and share across threads.
unsafe impl<T> Sync for Unspan<T> {}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use syn::Ident;

	#[test]
	fn works() {
		let foo: Ident = syn::parse_quote!(foo);
		let val = Unspan::new(&foo);

		let handle = std::thread::spawn(move || {
			// Access the span inside another thread
			val.to_token_stream().to_string()
		});

		let result = handle.join().unwrap();
		result.xpect_eq("foo");
	}
}
