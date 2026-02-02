mod action;
mod bundle_effect;
mod entity_target_event;
mod macros;
mod sendit;
mod to_tokens;
mod utils;
use macros::*;



/// Implements `TokenizeSelf` for a struct or enum.
/// All fields must also implement `TokenizeSelf`.
///
/// If the type is a struct with private fields, please use the `to_tokens` attribute to specify
/// a constructor accepting all fields in the order they are defined.
///
/// `TokenizeSelf` is implemented for primitives and some other common types,
/// please open a pr if you want to add support for a type in an external crate.
///
/// ## Example
///
/// ```rust ignore
/// #[derive(ToTokens)]
/// #[to_tokens(Foo::new)]
/// struct Foo{
///   bar: String,
/// }
///
/// impl Foo{
///   pub fn new(bar: String) -> Self {
///     Self { bar }
///   }
/// }
/// ```
#[proc_macro_derive(ToTokens, attributes(to_tokens, field))]
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
#[proc_macro_derive(BundleEffect)]
pub fn bundle_effect(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	bundle_effect::impl_bundle_effect(input).into()
}

/// Convenience helper to directly add observers to this entity.
/// This macro must be placed above `#[derive(Component)]` as it
/// sets the `on_add` hook.
/// ## Example
/// ```rust ignore
/// #[action(log_on_run)]
/// #[derive(Component)]
/// struct LogOnRun(pub String);
///
/// fn log_on_run(trigger: On<GetOutcome>, query: Populated<&LogOnRun>) {
/// 	let name = query.get(trigger.target()).unwrap();
/// 	println!("log_name_on_run: {}", name.0);
/// }
/// ```
#[proc_macro_attribute]
pub fn action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	action::impl_action(attr, item)
}


/// Macro for [`ActionEvent`]
///
/// ```ignore
/// #[derive(ActionEvent)]
/// /// Enable propagation using the given Traversal implementation
/// #[event(propagate = &'static ChildOf)]
/// /// Always propagate
/// #[event(auto_propagate)]
/// struct MyEvent;
/// ```
#[proc_macro_derive(ActionEvent, attributes(event))]
pub fn action_event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	entity_target_event::impl_action_event(input).into()
}
/// Macro for [`EntityTargetEvent`]
///
/// ```ignore
/// #[derive(EntityTargetEvent)]
/// /// Enable propagation using the given Traversal implementation
/// #[event(propagate = &'static ChildOf)]
/// /// Always propagate
/// #[event(auto_propagate)]
/// struct MyEvent;
/// ```
#[proc_macro_derive(EntityTargetEvent, attributes(event))]
pub fn entity_target_event(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	entity_target_event::impl_entity_target_event(input).into()
}

/// A unified macro for handling all test cases:
/// - sync native
/// - sync wasm
/// - async native
/// - async wasm
///
/// In the case of sync tests this simply replaces `#[beet::test]` with `#[test]`.
///
/// ## Parameters
///
/// - `timeout_ms`: Optional per-test timeout in milliseconds. Overrides suite-level timeout.
///
/// ```ignore
/// # use beet::prelude::*;
///
/// #[beet::test]
/// fn my_test() {
/// 	assert_eq!(2 + 2, 4);
/// }
///
/// #[beet::test]
/// async fn my_async_test() {
/// 	// some cross-platform async function 🥳
/// }
///
/// #[beet::test(timeout_ms = 100)]
/// async fn my_quick_test() {
/// 	// this test will timeout after 100ms
/// }
///
///
/// ```
#[proc_macro_attribute]
pub fn beet_test(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	parse_test_attr(attr, input)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}
