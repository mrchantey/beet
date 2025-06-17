use bevy::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use syn::Expr;
use syn::ExprClosure;
use syn::Ident;
use syn::Pat;
use syn::parse_quote;

/// Events are any attribute keys that start with `on`,
/// and the value is not a string literal.
/// This is to allow verbatim js handlers like `onclick="some_js_function()"`.
pub fn is_event(key: &str, value: &Expr) -> bool {
	key.starts_with("on") && !matches!(value, Expr::Lit(_))
}


pub fn tokenize_event_handler(
	key_str: &str,
	key_span: Span,
	mut value: Expr,
) -> Result<Expr> {
	let suffix = key_str.strip_prefix("on").unwrap_or(key_str);
	let event_ident =
		Ident::new(&format!("On{}", suffix.to_upper_camel_case()), key_span);

	parse_event_handler(&mut value, &event_ident);
	Ok(value)
}

/// if the tokens are a closure or a block where the last statement is a closure,
/// insert the matching [`Trigger`] type.
/// ie `<div onclick=|_|{ do_stuff() }/>` doesnt specify a type.
fn parse_event_handler(expr: &mut Expr, ident: &Ident) {
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

			// closures should be wrapped in a block
			// so we can safely call .into_node_bundle()
			// on the closure itsself
			*expr = syn::parse_quote! {{#closure}}
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
	fn test_parse_event_handler() {
		fn parse(val: TokenStream) -> String {
			let mut val = syn::parse2(val).unwrap();
			parse_event_handler(
				&mut val,
				&Ident::new("OnClick", Span::call_site()),
			);
			val.to_token_stream().to_string()
		}
		// leaves typed
		parse(quote! { |_: Trigger<WeirdType>| {} })
			.xpect()
			.to_be(quote! { {|_: Trigger<WeirdType>| {}} }.to_string());
		// inserts inferred
		parse(quote! { |foo| {} })
			.xpect()
			.to_be(quote! { {|foo: Trigger<OnClick>| {}} }.to_string());
		// inserts discard for empty
		parse(quote! { {|| {}} })
			.xpect()
			.to_be(quote! { {|_: Trigger<OnClick>| {}} }.to_string());
		// handles blocks
		parse(quote! { {|| {}} })
			.xpect()
			.to_be(quote! { {|_: Trigger<OnClick>| {}} }.to_string());
	}
}
