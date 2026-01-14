mod macros;
use macros::*;
use proc_macro::TokenStream;

/// A unified macro for handling all test cases:
/// - sync native
/// - sync wasm
/// - async native
/// - async wasm
///
/// In the case of sync tests this simply replaces `#[sweet::test]` with `#[test]`.
///
/// ## Parameters
///
/// - `timeout_ms`: Optional per-test timeout in milliseconds. Overrides suite-level timeout.
///
/// ```ignore
/// # use beet_core::prelude::*;
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
/// #[sweet::test(timeout_ms = 100)]
/// async fn my_quick_test() {
/// 	// this test will timeout after 100ms
/// }
///
///
/// ```
#[proc_macro_attribute]
pub fn test(attr: TokenStream, input: TokenStream) -> TokenStream {
	parse_sweet_test(attr, input)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}
