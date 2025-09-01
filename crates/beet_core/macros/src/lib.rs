#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
mod async_system;
mod impl_bundle;
mod sendit;
mod to_tokens;
mod utils;

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
#[proc_macro_derive(ImplBundle)]
pub fn impl_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	impl_bundle::impl_bundle(input).into()
}

/// Syntactic sugar for the Bevy [`AsyncComputeTaskPool`] pattern.
///
/// This macro rewrites async functions into synchronous Bevy systems by extracting
/// top-level `await` futures and streams into closure systems with the same parameters
/// scheduled after each future resolves.
/// ## Example
/// ```
///
/// #[derive(Resource)]
///	struct Count(usize);
///
/// #[async_system]
///	async fn my_future(mut count: ResMut<Count>) {
///		future::yield_now().await;
/// 	assert_eq!(count.0, 0);
///		count.0 += 1;
///		future::yield_now().await;
/// 	assert_eq!(count.0, 1);
///		count.0 += 1;
///		future::yield_now().await;
/// 	assert_eq!(count.0, 2);
///		count.0 += 1;
///	}
///
/// #[async_system]
///	async fn my_stream(mut count: ResMut<Count>) {
///		let mut stream = StreamCounter::new(3);
///		while let index = stream.next().await {
/// 		assert_eq!(count.0, index);
///			count.0 += 1;
///		}
///	}
/// ```
#[proc_macro_attribute]
pub fn async_system(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	async_system::async_system(attr, item).into()
}
