use crate::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use sweet::prelude::PipelineTarget;
use syn::Expr;
use syn::Ident;
use syn::Pat;
use syn::parse_quote;


/// [`SystemParam`] capable of finding all [`Attributes`] of a node,
/// collecting them into a [`TokenStream`].
#[derive(SystemParam)]
pub struct CollectNodeAttributes<'w, 's> {
	attributes: Query<'w, 's, &'static Attributes>,
	elements: Query<'w, 's, (), With<ElementNode>>,
	fragments: Query<'w, 's, (), With<FragmentNode>>,
	exprs_map: NonSend<'w, NonSendAssets<Expr>>,
	exprs: MaybeSpannedQuery<'w, 's, AttributeExpr>,
	keys: MaybeSpannedQuery<'w, 's, AttributeKeyExpr>,
	vals: MaybeSpannedQuery<'w, 's, AttributeValueExpr>,
	key_strs: MaybeSpannedQuery<'w, 's, AttributeKeyStr>,
	val_strs: MaybeSpannedQuery<'w, 's, AttributeValueStr>,
}

impl CollectCustomTokens for CollectNodeAttributes<'_, '_> {
	fn try_push_all(
		&self,
		spans: &NonSendAssets<proc_macro2::Span>,
		items: &mut Vec<proc_macro2::TokenStream>,
		entity: Entity,
	) -> Result<()> {
		let Ok(attributes) = self.attributes.get(entity) else {
			return Ok(());
		};
		if self.elements.contains(entity) {
			self.handle_element(spans, items, attributes)
		} else if self.fragments.contains(entity) {
			self.handle_fragment(spans, items, attributes, entity)
		} else {
			Ok(())
		}
	}
}

impl CollectNodeAttributes<'_, '_> {
	fn handle_element(
		&self,
		spans: &NonSendAssets<proc_macro2::Span>,
		entity_components: &mut Vec<TokenStream>,
		attributes: &Attributes,
	) -> Result<()> {
		let mut attr_entities = Vec::new();

		for attr_entity in attributes.iter() {
			let mut attr_components = Vec::new();
			// blocks ie <span {Vec3::new()} />
			// inserted directly as an entity component
			if let Some(attr) = self.maybe_spanned_expr(
				&self.exprs_map,
				spans,
				attr_entity,
				&self.exprs,
			)? {
				entity_components.push(attr);
			}

			if let Some(attr) = self.maybe_spanned_expr(
				&self.exprs_map,
				spans,
				attr_entity,
				&self.keys,
			)? {
				attr_components.push(quote! {AttributeKey::new(#attr)});
			}
			if let Some(attr) = self.maybe_spanned_expr(
				&self.exprs_map,
				spans,
				attr_entity,
				&self.vals,
			)? {
				if let Some(event_key) =
					self.try_event_key(spans, attr_entity)?
				{
					let attr = Self::try_insert_closure_type(attr, &event_key);

					// in the case of an event the value is an observer added to the parent
					entity_components.push(quote! {
						EntityObserver::new::<#event_key,_,_,_>(#attr)
					});
				} else {
					attr_components.push(quote! {AttributeValue::new(#attr)});
				}
			}

			self.try_push_custom(
				spans,
				&mut attr_components,
				attr_entity,
				&self.key_strs,
			)?;
			self.try_push_custom(
				spans,
				&mut attr_components,
				attr_entity,
				&self.val_strs,
			)?;
			if attr_components.len() == 1 {
				attr_entities.push(attr_components.pop().unwrap());
			} else if !attr_components.is_empty() {
				attr_entities.push(quote! {
					(#(#attr_components),*)
				});
			}
		}
		if !attr_entities.is_empty() {
			entity_components.push(quote! {
				related!(Attributes[
				#(#attr_entities),*
			])
					});
		}
		Ok(())
	}

	fn try_event_key(
		&self,
		spans: &NonSendAssets<Span>,
		entity: Entity,
	) -> Result<Option<Ident>> {
		let Ok((str, span)) = self.key_strs.get(entity) else {
			return Ok(None);
		};
		let Some(suffix) = str.strip_prefix("on") else {
			return Ok(None);
		};
		let span = if let Some(span) = span {
			spans.get(span).map(|s| *s)?
		} else {
			Span::call_site()
		};

		let suffix = ToUpperCamelCase::to_upper_camel_case(suffix);

		Ident::new(&format!("On{suffix}"), span).xsome().xok()
	}

	/// if the tokens are a closure, insert the matching [`Trigger`] type
	fn try_insert_closure_type(
		tokens: TokenStream,
		ident: &Ident,
	) -> TokenStream {
		if let Ok(Expr::Closure(mut closure)) =
			syn::parse2(tokens.clone())
		{
			if let Some(first_param) = closure.inputs.first_mut() {
				match &*first_param {
					Pat::Type(_) => {
						// Already has type annotation, leave as is
					}
					pat => {
						let pat_clone = pat.clone();
						// insert type
						*first_param = Pat::Type(
							parse_quote! {#pat_clone:Trigger<#ident>},
						);
					}
				};
				closure.to_token_stream()
			} else {
				// If no parameters, add one with discard name
				closure
					.inputs
					.push(Pat::Type(parse_quote!(_:Trigger<#ident>)));
				closure.to_token_stream()
			}
		} else {
			// Not a closure, return unchanged
			tokens
		}
	}

	#[allow(unused)]
	fn handle_fragment(
		&self,
		spans: &NonSendAssets<proc_macro2::Span>,
		items: &mut Vec<TokenStream>,
		attributes: &Attributes,
		entity: Entity,
	) -> Result<()> {
		Ok(())
	}
}

/// An attribute key represented as tokens, usually either a string literal or a block.
///
/// ## Example
/// ```ignore
/// let key = "hidden";
/// rsx!{<span {key}=true />};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeKeyExpr(NonSendHandle<Expr>);
impl AttributeKeyExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}


/// The tokens for an attribute value, usually a block or a literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeValueExpr(NonSendHandle<Expr>);


impl AttributeValueExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}


/// Tokens for an attribute without a specified key-value pair.
/// This is known as the spread attribute in JSX, although rust
/// apis dont require the `...` prefix.
/// ## Example
/// ```ignore
/// rsx!{<span {props} />};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, Component)]
#[component(immutable)]
pub struct AttributeExpr(NonSendHandle<Expr>);


impl AttributeExpr {
	pub fn new(value: NonSendHandle<Expr>) -> Self { Self(value) }
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::Span;
	use quote::quote;
	use sweet::prelude::*;
	use syn::Ident;

	#[test]
	fn insert_closure_type() {
		// leaves typed
		CollectNodeAttributes::try_insert_closure_type(
			quote! { |_: Trigger<WeirdType>| {} },
			&Ident::new("OnClick", Span::call_site()),
		)
		.to_string()
		.xpect()
		.to_be(quote! { |_: Trigger<WeirdType>| {} }.to_string());
		// inserts inferred
		CollectNodeAttributes::try_insert_closure_type(
			quote! { |foo| {} },
			&Ident::new("OnClick", Span::call_site()),
		)
		.to_string()
		.xpect()
		.to_be(quote! { |foo: Trigger<OnClick>| {} }.to_string());
		// inserts discard for empty
		CollectNodeAttributes::try_insert_closure_type(
			quote! { || {} },
			&Ident::new("OnClick", Span::call_site()),
		)
		.to_string()
		.xpect()
		.to_be(quote! { |_: Trigger<OnClick>| {} }.to_string());
	}
}
