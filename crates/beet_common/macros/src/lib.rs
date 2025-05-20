#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
mod to_tokens;

/// Implements `IntoCustomTokens` for a struct or enum.
/// All fields must also implement `IntoCustomTokens`, please open
/// a pr if you want to add support for a type.
///
/// ## Example
///
/// ```rust ignore
/// #[derive(ToTokens)]
/// struct Foo{
///   bar: String,
/// }
/// ```
#[proc_macro_derive(ToTokens, attributes(field))]
pub fn derive_to_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	to_tokens::impl_derive_to_tokens(input).into()
}
