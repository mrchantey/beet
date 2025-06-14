#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
mod non_send_component;
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
/// Implements `IntoCustomTokens` for a struct or enum.
/// All fields must also implement `IntoCustomTokens`, please open
/// a pr if you want to add support for a type.
///
/// ## Example
///
/// ```rust ignore
/// #[derive(NonSendComponent)]
/// struct Foo{
/// 	// some non-send field
///   bar: RefCell<String>,
/// }
/// ```
#[proc_macro_derive(NonSendComponent, attributes(field))]
pub fn derive_non_send_component(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	non_send_component::impl_non_send_component(input).into()
}
