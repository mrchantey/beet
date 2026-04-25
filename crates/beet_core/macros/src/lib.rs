#![no_std]
extern crate alloc;
mod action;
mod as_any;
mod bundle_effect;
mod entity_target_event;
mod from_tokens;
mod getset;
mod main_attr;
mod mdx;
#[cfg(feature = "rsx")]
mod rsx;
mod sendit;
mod test_attr;
mod to_tokens;


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
/// Derive [`FromTokens`] for a struct, generating a companion `*Tokens` struct.
///
/// Fields marked with `#[token]` will be replaced by [`Token`] in the generated
/// `*Tokens` struct. Doc comments from the original struct are copied to `*Tokens`.
/// The generated struct always derives `Debug, Clone, PartialEq, Reflect` — the
/// minimum set required to satisfy `type Tokens: Typed + FromReflect`.
///
/// > **Note:** rustc strips `#[derive(...)]` from `input.attrs` before invoking
/// > derive proc macros, so derives cannot be copied from the original struct.
///
/// ## Example
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Reflect, FromTokens)]
/// pub struct Motion {
///     #[token]
///     pub duration: Duration,
///     pub ease: EaseFunction,
/// }
/// // Generates MotionTokens { duration: Token, ease: EaseFunction }
/// // and impl FromTokens for Motion { ... }
/// ```
#[proc_macro_derive(FromTokens, attributes(token))]
pub fn derive_from_tokens(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	from_tokens::impl_from_tokens(input)
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

/// Implements `AsAny` for a struct or enum, allowing it to be downcast at runtime.
#[proc_macro_derive(Any, attributes(event))]
pub fn as_any(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	as_any::impl_as_any(input).into()
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
	test_attr::impl_test_attr(attr, input)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}

/// MDX-style markdown macro with `{}` interpolation.
///
/// Parses markdown text interspersed with `{}` bundle expressions.
/// The crate path is resolved automatically via `internal_or_beet`.
///
/// # Input Format
///
/// ```text
/// mdx!(# Heading text {bundle_expr} more text)
/// mdx!("string with {interpolation}")
/// ```
#[proc_macro]
pub fn mdx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	mdx::impl_mdx(input)
}

/// JSX-like macro for constructing Bevy ECS bundles from HTML-like syntax.
///
/// Lowercase tags become [`Element`] bundles, capitalized tags become
/// component constructors using `Default + SetWith` patterns.
///
/// ## Example
///
/// ```rust ignore
/// fn my_ui() -> impl Bundle {
///     rsx!{
///         <div class="container">
///             <span>"hello"</span>
///         </div>
///     }
/// }
/// ```
#[cfg(feature = "rsx")]
#[proc_macro]
pub fn rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	rsx::impl_rsx(input)
}


/// Entry point macro for async `main` functions, using [`async_executor::LocalExecutor`].
///
/// Works like a standard async main entry point but uses `async-executor` for a lightweight, dependency-light runtime.
///
/// # Requirements
///
/// - Must be applied to an `async fn main()`
/// - Not supported on `wasm32` targets
///
/// # Example
///
/// ```ignore
/// #[beet::main]
/// async fn main() {
///     // async code here
/// }
///
/// #[beet::main]
/// async fn main() -> anyhow::Result<()> {
///     // async code that returns a Result
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn beet_main(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	main_attr::impl_main_attr(attr, input)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}

#[proc_macro_attribute]
pub fn action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	action::impl_action(attr, item)
}

/// Generate getter methods for struct fields.
///
/// All methods are `pub` by default and return `&T`.
///
/// ## Struct-level attributes
///
/// - `#[get(clone)]` - return `T` via `.clone()` by default
/// - `#[get(copy)]` - return `T` by copy by default
/// - `#[get(vis = private)]` - set default visibility
/// - `#[get(unwrap_trait)]` - unwrap `Box<dyn Trait>` / `Arc<dyn Trait>`
///
/// ## Field-level attributes
///
/// - `#[get(skip)]` - skip this field
/// - `#[get(clone)]`, `#[get(copy)]`, `#[get(vis = pub_crate)]` - override per field
/// - `#[get(unwrap_trait)]` - unwrap trait wrapper for this field
///
/// ```ignore
/// #[derive(Get)]
/// #[get(copy)]
/// pub struct Point {
///     x: f32,
///     #[get(clone, vis = private)]
///     name: String,
/// }
/// ```
#[proc_macro_derive(Get, attributes(get))]
pub fn derive_get(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	getset::get::impl_get(input)
}

/// Generate mutable getter methods for struct fields.
///
/// All methods are `pub` by default and return `&mut T`.
/// Method names follow the `field_mut` convention.
///
/// ```ignore
/// #[derive(GetMut)]
/// pub struct Foo {
///     #[get_mut(vis = pub_crate)]
///     name: String,
///     #[get_mut(skip)]
///     secret: String,
/// }
/// ```
#[proc_macro_derive(GetMut, attributes(get_mut))]
pub fn derive_get_mut(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	getset::get_mut::impl_get_mut(input)
}

/// Generate setter methods for struct fields.
///
/// All methods are `pub` by default, take `&mut self`, and return `&mut Self`.
/// Method names follow the `set_field` convention.
///
/// ## Options
///
/// - `unwrap_option` - accept `T` instead of `Option<T>`, wrapping with `Some`
/// - `unwrap_trait` - accept `impl Trait` for `Box<dyn Trait>` / `Arc<dyn Trait>`
///
/// ```ignore
/// #[derive(Set)]
/// pub struct Config {
///     name: String,
///     #[set(unwrap_option)]
///     label: Option<String>,
///     #[set(unwrap_trait)]
///     handler: Box<dyn Handler>,
/// }
/// ```
#[proc_macro_derive(Set, attributes(set))]
pub fn derive_set(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	getset::set::impl_set(input)
}

/// Generate builder-style setter methods for struct fields.
///
/// All methods are `pub` by default, take `mut self`, and return `Self`.
/// Method names follow the `with_field` convention.
///
/// ## Options
///
/// - `unwrap_option` - accept `T` instead of `Option<T>`, wrapping with `Some`
/// - `unwrap_trait` - accept `impl Trait` for `Box<dyn Trait>` / `Arc<dyn Trait>`
///
/// ```ignore
/// #[derive(SetWith)]
/// pub struct Config {
///     name: String,
///     #[set_with(unwrap_option)]
///     label: Option<String>,
/// }
///
/// let config = Config::default().with_name("hello".into()).with_label("world".into());
/// ```
#[proc_macro_derive(SetWith, attributes(set_with))]
pub fn derive_set_with(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	getset::set_with::impl_set_with(input)
}
