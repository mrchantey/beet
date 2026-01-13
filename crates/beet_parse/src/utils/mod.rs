mod lit;
use beet_core::prelude::*;
pub use lit::*;
use proc_macro2::TokenStream;


/// beet imports for use in blocks and nested functions,
/// top level imports should be more specific.
/// This will be `use beet::prelude::*;` if external crate,
/// otherwise uses internal names
pub fn dom_imports() -> TokenStream {
	if pkg_ext::is_internal() {
		quote::quote! {
			#[allow(unused)]
			use beet_dom::prelude::*;
			#[allow(unused)]
		}
	} else {
		quote::quote! {
			use beet::prelude::*;
		}
	}
}
