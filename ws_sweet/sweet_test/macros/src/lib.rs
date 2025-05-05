mod macros;
use macros::*;
use proc_macro::TokenStream;
use sweet_test_attr::SweetTestAttr;

/// A unified macro for handling all test cases:
/// - sync native
/// - sync wasm
/// - async native
/// - async wasm
///
/// In the case of sync tests this simply replaces `#[sweet::test]` with `#[test]`.
///
/// ```ignore
/// # use sweet_test::as_sweet::*;
///
/// #[sweet::test]
/// fn my_test() {
/// 	assert_eq!(2 + 2, 4);
/// }
///
/// #[sweet::test]
/// async fn my_async_test() {
/// 	// some cross-platform async function 🥳
/// }
///
///
/// ```
#[proc_macro_attribute]
pub fn test(attr: TokenStream, input: TokenStream) -> TokenStream {
	SweetTestAttr::parse(attr, input)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}
