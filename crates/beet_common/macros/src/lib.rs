#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
mod impl_bundle;
mod sendit;
mod to_tokens;
mod utils;

/// Implements `TokenizeSelf` for a struct or enum.
/// All fields must also implement `TokenizeSelf`, please open
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
/// #[sendit(derive(Clone))]
/// struct Foo{
/// 	// some non-send field
///   bar: RefCell<String>,
/// }
///
/// /*
/// The above will generate the following code:
/// #[derive(Clone)]
/// struct FooSendit(pub send_wrapper::SendWrapper<Foo>);
/// */
/// ```
#[proc_macro_derive(Sendit, attributes(sendit))]
pub fn derive_sendit(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	sendit::impl_sendit(input).into()
}

/// Implement [`Bundle`] for a struct that implements [`BundleEffect`].
///
/// ## Example
///
/// ```rust ignore
/// #[derive(BundleEffect)]
/// struct Foo{
///   bar: String,
/// }
/// impl BundleEffect for Foo {
///		fn apply(self, entity: &mut EntityWorldMut) { entity.insert(Bar); }
///	}
/// ```
#[proc_macro_derive(ImplBundle)]
pub fn impl_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	impl_bundle::impl_bundle(input).into()
}
