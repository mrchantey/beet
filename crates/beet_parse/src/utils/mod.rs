mod lit;
use beet_utils::prelude::*;
pub use lit::*;
use proc_macro2::TokenStream;


/// `use beet::prelude::*;` if external, otherwise uses
/// internal names
pub fn dom_imports() -> TokenStream {
	if pkg_ext::is_internal() {
		quote::quote! {
			// including in beet_rsx causes signals to stop working??
			// i think it screws up bevy type_name system
			// #[allow(unused)]
			// use beet_rsx::prelude::*;
			#[allow(unused)]
			use beet_dom::prelude::*;
			#[allow(unused)]
			use beet_core::prelude::*;
			#[allow(unused)]
			use beet_utils::prelude::*;
		}
	} else {
		quote::quote! {
			use beet::prelude::*;
		}
	}
}
