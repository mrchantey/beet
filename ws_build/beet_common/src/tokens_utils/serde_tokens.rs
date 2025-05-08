use proc_macro2::TokenStream;




/// A trait for types that can be tokenized into
/// a Rust representation.
pub trait RustTokens {
	/// Convert the type into a rust [`TokenStream`] for use in
	/// rust files like macros, ie `MyStruct::new(7)`.
	/// It assumes the type is in scope.
	fn into_rust_tokens(&self) -> TokenStream;
}
/// A trait for types that can be tokenized into both
/// a Rust and a [`ron`] representation.
pub trait RonTokens: RustTokens {
	/// Convert the type into a ron [`TokenStream`], ie `MyStruct(7)`.
	fn into_ron_tokens(&self) -> TokenStream;
}
