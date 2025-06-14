#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
mod sendit;
mod to_tokens;
mod utils;

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
/// Creates a [SendWrapper](send_wrapper::SendWrapper) newtype that implements `Send` for a struct or enum.
///
/// ## Example
///
/// ```rust ignore
/// #[derive(Sendit)]
/// struct Foo{
/// 	// some non-send field
///   bar: RefCell<String>,
/// }
///
/// /*
/// struct FooSend(pub send_wrapper::SendWrapper<Foo>);
/// */
///
/// ```
#[proc_macro_derive(Sendit, attributes(sendit))]
pub fn derive_sendit(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	sendit::impl_sendit(input).into()
}
