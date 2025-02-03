use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

/// Instructions for converting the rust parts of an rsx node to tokens.
///
/// By default the ident is used to call specific transform methods,
/// if using this be sure to implement the following:
/// - map_node_block
/// - map_attribute_block
/// - map_attribute_value
/// - map_event
pub trait RsxRustTokens {
	fn ident() -> TokenStream;

	/// must return a valid RsxNode, ie RsxNode::Block
	fn map_node_block(block: &TokenStream) -> TokenStream {
		let ident = Self::ident();
		quote! {#ident::map_node_block(#block)}
	}

	/// This should return an [RsxAttribute::Block](crate::prelude::RsxAttribute::Block)
	/// but can technically be any valid RsxAttribute or comma seperated list of RsxAttributes
	fn register_attribute_block(block: &TokenStream) -> TokenStream {
		let ident = Self::ident();
		quote! {#ident::map_attribute_block(#block)}
	}

	/// This should return an [RsxAttribute::BlockValue](crate::prelude::RsxAttribute::BlockValue)
	/// but can technically be any valid RsxAttribute or comma seperated list of RsxAttributes
	fn map_attribute_value(key: &str, value: &TokenStream) -> TokenStream {
		let ident = Self::ident();
		quote! {#ident::map_attribute_value(#key, #value)}
	}

	/// Events are a special case, they are handled by the event registry not the
	/// reactive framework.
	/// This should return an [RsxAttribute::BlockValue](crate::prelude::RsxAttribute::BlockValue)
	/// but can technically be any valid RsxAttribute or comma seperated list of RsxAttributes
	fn map_event(key: &str, value: &TokenStream) -> TokenStream {
		let key = key.to_string();

		let register_func =
			syn::Ident::new(&format!("register_{key}"), value.span());
		quote! {
			RsxAttribute::BlockValue {
				key: #key.to_string(),
				initial: "needs-event-cx".to_string(),
				effect: Box::new(move |cx| {
					::#register_func(#key,cx,#value);
				}),
			}
		}
	}
}
