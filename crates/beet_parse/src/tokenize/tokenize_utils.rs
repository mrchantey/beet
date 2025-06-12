use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;

/// Define a function for tokenizing each listed component, all of which
/// must implement [`IntoCustomTokens`].
macro_rules! tokenize_maybe_spanned {
		($name:ident,$($type:ty),* $(,)?) => {
			pub fn $name(
				world: &World,
				items: &mut Vec<TokenStream>,
				entity: Entity
			) -> Result<()> {
				$(
					if let Some(value) = tokenize_maybe_spanned::<$type>(world, entity)? {
						items.push(value);
					}
				)*
				Ok(())
			}
	};
}

// pub fn maybe_push(items: &mut Vec<TokenStream>, item: Option<TokenStream>) {
// 	if let Some(item) = item {
// 		items.push(item);
// 	}
// }

pub fn maybe_tuple(items: Vec<TokenStream>) -> TokenStream {
	if items.is_empty() {
		().into_custom_token_stream()
	} else if items.len() == 1 {
		items.into_iter().next().unwrap()
	} else {
		quote! { (#(#items),*) }
	}
}

tokenize_maybe_spanned![
	tokenize_rsx_nodes,
	NodeTag,
	FragmentNode,
	TemplateNode,
	TextNode,
	BlockNode
];

tokenize_maybe_spanned!(
	tokenize_web_nodes,
	DoctypeNode,
	CommentNode,
	ElementNode,
);

#[rustfmt::skip]
tokenize_maybe_spanned!(
	tokenize_rsx_directives, 
	SlotChild, 
	SlotTarget
);

tokenize_maybe_spanned!(
	tokenize_web_directives,
	HtmlHoistDirective,
	ClientLoadDirective,
	ClientOnlyDirective,
	LangContent
);


pub(super) fn tokenize_maybe_spanned<T: Component + IntoCustomTokens>(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	let entity = world.entity(entity);
	match (
		entity.get::<T>(),
		entity.get::<ItemOf<T, SendWrapper<Span>>>(),
	) {
		(Some(value), Some(span)) => {
			let value = value.into_custom_token_stream();
			Ok(Some(quote::quote_spanned! { ***span =>
				#value
			}))
		}
		(Some(value), None) => Ok(Some(value.into_custom_token_stream())),
		_ => Ok(None),
	}
}

/// Get an expression from the entity, if it exists, and wrap in the
/// corresponding `span` if it exists.
pub(super) fn maybe_spanned_expr<
	T: Component + std::ops::Deref<Target = SendWrapper<Expr>>,
>(
	world: &World,
	entity: Entity,
) -> Result<Option<Expr>> {
	let entity = world.entity(entity);
	match (
		entity.get::<T>(),
		entity.get::<ItemOf<T, SendWrapper<Span>>>(),
	) {
		(Some(value), Some(span)) => {
			let value = &***value;
			// let value = value.into_custom_token_stream();
			Ok(Some(syn::parse_quote_spanned! { ***span =>
				#value
			}))
		}
		(Some(value), None) => Ok(Some((***value).clone())),
		_ => Ok(None),
	}
}
