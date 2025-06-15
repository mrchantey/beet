#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(proc_macro_span)]
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::ItemFn;
use syn::parse_macro_input;

/// Parse [`rsmtl`] tokens into a [`Bundle`].
/// ```ignore
/// let node = rsx! {<div> the value is {3}</div>};
/// ```
///
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
	let source_file = source_file(&tokens);
	// this method creates a new app for every rstml macro,
	// we may find it faster to reuse a single app, although
	// parallelism will still be tricky because tokens are non-send
	tokenize_rstml(tokens.into(), source_file)
		.unwrap_or_else(err_tokens)
		.into()
}

/// Mostly used for testing, this macro expands to an [`WebNodeTemplate`]
#[proc_macro]
pub fn rsx_template(_tokens: TokenStream) -> TokenStream { todo!() }


/// Mostly used for testing, this macro expands to [`WebTokens`]
#[proc_macro]
pub fn rsx_tokens(tokens: TokenStream) -> TokenStream {
	let source_file = source_file(&tokens);
	tokenize_rstml_tokens(tokens.into(), source_file)
		.unwrap_or_else(err_tokens)
		.into()
}

/// Adds a builder pattern to a struct enabling construction as an
/// rsx component
#[proc_macro_derive(Props, attributes(node, field))]
pub fn derive_props(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse_derive_props(input).into()
}


/// Mark a function as a template function.
///
/// ## Example
///
/// ```rust ignore
/// #[template]
/// fn MyTemplate(hidden:bool) -> impl Bundle {
/// 	rsx!{<div hidden={hidden}>hello world</div>}
/// }
/// ```
#[proc_macro_attribute]
pub fn template(
	_attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as ItemFn);
	template_func(input).into()
}


/// Allow a struct to be included as a `#[field(flatten)]` of another struct
#[proc_macro_derive(Buildable, attributes(field))]
pub fn derive_buildable(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse_derive_buildable(input).into()
}
/// Implements [`IntoTemplateBundle`] for a struct with named fields,
/// where each field also implements [`IntoTemplateBundle`].
#[proc_macro_derive(TemplateBundle, attributes(field))]
pub fn derive_template_bundle(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	impl_into_template_bundle(input).into()
}

fn err_tokens(err: impl ToString) -> proc_macro2::TokenStream {
	let err = err.to_string();
	quote::quote! {
		compile_error!(#err);
	}
}


/// For a token stream create the [`FileSpan`] using its location.
/// we'll get this from proc_macro2::Span::source_file, when this issue resolves:
/// https://github.com/dtolnay/proc-macro2/issues/499
fn source_file(tokens: &proc_macro::TokenStream) -> WorkspacePathBuf {
	// cloning is cheap, its an immutable arc
	tokens
		.clone()
		.into_iter()
		.next()
		.map(|token| WorkspacePathBuf::new(token.span().file()))
		.unwrap_or_default()
}
