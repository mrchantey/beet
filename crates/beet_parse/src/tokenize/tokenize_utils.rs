use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;


/// bundle impl limit
const BOUNDED_MAX: usize = 12;

/// Uses `related!` if the number of related items can be represented as a tuple,
/// otherwise use [`spawn_with`]
pub fn unbounded_related(
	ident: &Ident,
	related: Vec<TokenStream>,
) -> TokenStream {
	if related.len() <= BOUNDED_MAX {
		quote! { related!{#ident [#(#related),*]} }
	} else {
		quote! {spawn_with::<#ident,_>(move |parent| {
			#(parent.spawn(#related);)*
		})}
	}
}


/// Uses Tuple if the number of items items can be represented as a tuple,
/// otherwise use [`OnSpawn`]
pub fn unbounded_bundle(items: Vec<TokenStream>) -> TokenStream {
	if items.is_empty() {
		().self_token_stream()
	} else if items.len() == 1 {
		items.into_iter().next().unwrap()
	} else if items.len() <= BOUNDED_MAX {
		quote! { (#(#items),*) }
	} else {
		quote! {OnSpawn::new(move |entity| {
			#(entity.insert(#items);)*
		})}
	}
}


/// Return the [`AttributeKey`] if it exists,
/// and its span or [`Span::call_site()`].
pub(super) fn maybe_spanned_attr_key(
	world: &World,
	entity: Entity,
) -> Option<(String, Span)> {
	let entity = world.entity(entity);
	match (
		entity.get::<AttributeKey>(),
		entity.get::<SpanOf<AttributeKey>>(),
	) {
		(Some(key), Some(span)) => Some((key.to_string(), span.clone().take())),
		(Some(key), None) => Some((key.to_string(), Span::call_site())),
		_ => None,
	}
}
