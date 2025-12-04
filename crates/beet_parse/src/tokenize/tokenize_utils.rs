use beet_core::prelude::*;
use beet_dom::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;


const BOUNDED_MAX: usize = 12;

pub fn unbounded_related<T: TypePath>(
	related: Vec<TokenStream>,
) -> Result<TokenStream> {
	let ident = type_path_to_ident::<T>()?;
	unbounded_related_ident(&ident, related).xok()
}

// solved in bevy 0.17?
/// Uses `related!` if the number of related items can be represented as a tuple,
/// otherwise use [`spawn_with`]
pub fn unbounded_related_ident(
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

fn type_path_to_ident<T: TypePath>() -> Result<Ident> {
	let ident = T::type_ident().ok_or_else(|| {
		bevyhow!(
			"Failed to get type identifier for component: {}",
			std::any::type_name::<T>()
		)
	})?;
	let ident: Ident = syn::parse_str(ident).map_err(|_| {
		bevyhow!(
			"Failed to parse type identifier for component: {}",
			std::any::type_name::<T>()
		)
	})?;

	Ok(ident)
}
