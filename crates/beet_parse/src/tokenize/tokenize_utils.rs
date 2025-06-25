use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;
use syn::Ident;

/// Gets the first 'expression' for an attribute, searching in the following order:
/// - [`AttributeExpr`]
/// - [`tokenize_combinator_exprs`]
pub fn first_attribute_expr(
	world: &World,
	attr_entity: Entity,
) -> Result<Option<Expr>> {
	if let Some(attr) = maybe_spanned_expr::<AttributeExpr>(world, attr_entity)?
	{
		Ok(Some(attr))
	} else if let Some(combinator) =
		tokenize_combinator_exprs(world, attr_entity)?
	{
		syn::parse2(combinator).map(Some).map_err(Into::into)
	} else {
		Ok(None)
	}
}


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

/// Define a function for tokenizing each listed component, all of which
/// must implement [`TokenizeSelf`].
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


tokenize_maybe_spanned![
	tokenize_idxs,
	MacroIdx,
	ExprIdx,
];
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
	StyleScope,
	StyleCascade,
	LangContent
);


pub(super) fn tokenize_maybe_spanned<T: Component + TokenizeSelf>(
	world: &World,
	entity: Entity,
) -> Result<Option<TokenStream>> {
	let entity = world.entity(entity);
	match (
		entity.get::<T>(),
		entity.get::<ItemOf<T, SendWrapper<Span>>>(),
	) {
		(Some(value), Some(span)) => {
			let value = value.self_token_stream();
			Ok(Some(quote::quote_spanned! { ***span =>
				#value
			}))
		}
		(Some(value), None) => Ok(Some(value.self_token_stream())),
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
			// let value = value.self_token_stream();
			Ok(Some(syn::parse_quote_spanned! { ***span =>
				#value
			}))
		}
		(Some(value), None) => Ok(Some((***value).clone())),
		_ => Ok(None),
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
		entity.get::<ItemOf<AttributeKey, SendWrapper<Span>>>(),
	) {
		(Some(key), Some(span)) => {
			Some((key.to_string(), span.clone().take().take()))
		}
		(Some(key), None) => Some((key.to_string(), Span::call_site())),
		_ => None,
	}
}
