use crate::tokenize::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use syn::Expr;
use syn::ExprClosure;
use syn::Ident;
use syn::Pat;
use syn::parse_quote;


/// If the attribute matches the requirements for an event observer,
/// append to the `entity_components` and return `Ok(true)`.
///
/// ## Requirements
/// - Key is a string literal starting with `on`
/// - Value is not a string, (allows for verbatim js handlers)
pub fn try_event_observer(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<bool> {
	let Some(mut attr) =
		maybe_spanned_expr::<AttributeValueExpr>(world, entity)?
	else {
		return Ok(false);
	};

	let entity = world.entity(entity);

	let Some(lit) = entity.get::<AttributeLit>() else {
		return Ok(false);
	};
	// If value is a string literal, we shouldn't process it as an event handler,
	// to preserve onclick="some_js_function()"
	if lit.value.is_some() {
		return Ok(false);
	}

	let Some(suffix) = lit.key.strip_prefix("on") else {
		return Ok(false);
	};

	let span = entity
		.get::<ItemOf<AttributeKeyExpr, SendWrapper<Span>>>()
		.map(|s| ***s)
		.unwrap_or(Span::call_site());

	let suffix = ToUpperCamelCase::to_upper_camel_case(suffix);

	let event_ident = Ident::new(&format!("On{suffix}"), span);
	let lit_key_str = &lit.key;

	try_insert_closure_type(&mut attr, &event_ident);
	entity_components.push(quote! {EventObserver::new(#lit_key_str)});
	entity_components
		.push(quote! {EntityObserver::new(#[allow(unused_braces)]#attr)});
	Ok(true)
}

/// if the tokens are a closure or a block where the last statement is a closure,
/// insert the matching [`Trigger`] type.
/// ie `<div onclick=|_|{ do_stuff() }/>` doesnt specify a type.
fn try_insert_closure_type(expr: &mut Expr, ident: &Ident) {
	fn process_closure(closure: &mut ExprClosure, ident: &Ident) {
		match closure.inputs.first_mut() {
			Some(first_param) => match &*first_param {
				Pat::Type(_) => {
					// Already has type annotation, leave as is
				}
				pat => {
					let pat_clone = pat.clone();
					// insert type
					*first_param =
						Pat::Type(parse_quote! {#pat_clone:Trigger<#ident>});
				}
			},
			None => {
				// If no parameters, add one with discard name
				closure
					.inputs
					.push(Pat::Type(parse_quote!(_:Trigger<#ident>)));
			}
		};
	}

	match expr {
		Expr::Closure(closure) => {
			process_closure(closure, ident);
		}
		Expr::Block(block) => {
			// Handle the case where a block's last statement is a closure
			if let Some(last_stmt) = block.block.stmts.last_mut() {
				if let syn::Stmt::Expr(Expr::Closure(closure), _) = last_stmt {
					process_closure(closure, ident);
				}
			}
			// Block doesn't end with a closure, return unchanged
		}
		_ => {
			// Not a closure or block, unchanged
		}
	}
}




#[cfg(test)]
mod test {
	use super::*;
	use proc_macro2::Span;
	use proc_macro2::TokenStream;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;
	use syn::Ident;

	#[test]
	fn insert_closure_type() {
		fn parse(val: TokenStream) -> String {
			let mut val = syn::parse2(val).unwrap();
			try_insert_closure_type(
				&mut val,
				&Ident::new("OnClick", Span::call_site()),
			);
			val.to_token_stream().to_string()
		}
		// leaves typed
		parse(quote! { |_: Trigger<WeirdType>| {} })
			.xpect()
			.to_be(quote! { |_: Trigger<WeirdType>| {} }.to_string());
		// inserts inferred
		parse(quote! { |foo| {} })
			.xpect()
			.to_be(quote! { |foo: Trigger<OnClick>| {} }.to_string());
		// inserts discard for empty
		parse(quote! { || {} })
			.xpect()
			.to_be(quote! { |_: Trigger<OnClick>| {} }.to_string());
		// handles blocks
		parse(quote! { {|| {}} })
			.xpect()
			.to_be(quote! { {|_: Trigger<OnClick>| {}} }.to_string());
	}
}
