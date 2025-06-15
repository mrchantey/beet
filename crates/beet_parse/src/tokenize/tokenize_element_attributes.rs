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



pub fn tokenize_element_attributes(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<ElementNode>() {
		Ok(())
	} else if let Some(attrs) = entity.get::<Attributes>() {
		let mut attr_entities = Vec::new();
		for attr_entity in attrs.iter() {
			if try_event_observer(world, entity_components, attr_entity)? {
				// events are handled separately
				continue;
			}

			let mut attr_components = Vec::new();
			// blocks ie <span {Vec3::new()} />
			// inserted directly as an entity component
			if let Some(attr) =
				maybe_spanned_expr::<AttributeExpr>(world, attr_entity)?
			{
				entity_components.push(quote! {#attr.into_node_bundle()});
			}

			if let Some(attr) =
				maybe_spanned_expr::<AttributeKeyExpr>(world, attr_entity)?
			{
				attr_components.push(quote! {#attr.into_attr_key_bundle()});
			}
			if let Some(attr) =
				maybe_spanned_expr::<AttributeValueExpr>(world, attr_entity)?
			{
				attr_components.push(quote! {#attr.into_attr_val_bundle()});
			}
			if let Some(attr) = tokenize_combinator_exprs(world, attr_entity)? {
				if world.entity(attr_entity).contains::<AttributeKeyExpr>() {
					// if this attribute has a key, the combinator must be a value
					attr_components.push(quote! {#attr.into_attr_val_bundle()});
				} else {
					// otherwise the combinator is a block value, aka a component
					entity_components.push(attr);
				}
			}
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
	} else {
		Ok(())
	}
}


/// If the attribute matches the requirements for an event observer,
/// append to the `entity_components` and return `Ok(true)`.
///
/// ## Requirements
/// - Key is a string literal starting with `on`
/// - Value is not a string, (allows for verbatim js handlers)
fn try_event_observer(
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
	use crate::prelude::*;
	use crate::tokenize::tokenize_element_attributes::try_insert_closure_type;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
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

	fn parse(tokens: TokenStream) -> Matcher<String> {
		tokenize_rstml(tokens, WsPathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	#[test]
	fn key_value() {
		quote! {
			<span hidden/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[
					"hidden".into_attr_key_bundle()
				])
			)}
			.to_string(),
		);
		quote! {
			<span hidden=true/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[(
					"hidden".into_attr_key_bundle(),
					true.into_attr_val_bundle()
				)])
			)}
			.to_string(),
		);
	}
	#[test]
	fn block() {
		quote! {
			<span {foo}/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				{foo}.into_node_bundle()
			)}
			.to_string(),
		);
	}
	#[test]
	fn events() {
		quote! {
			<span onclick={foo}/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				EventObserver::new("onclick"),
				EntityObserver::new(#[allow(unused_braces)]{foo})
			)}
			.to_string(),
		);
		quote! {
			<span onclick="some_js_func"/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				related!(Attributes[(
					"onclick".into_attr_key_bundle(),
					"some_js_func".into_attr_val_bundle()
				)])
			)}
			.to_string(),
		);
	}
	#[test]
	fn all() {
		quote! {
			<span
				hidden
				class="foo"
				{foo}
				onmousemove="some_js_func"
				onclick=|| { println!("clicked"); }
			/>
		}
		.xmap(parse)
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: true },
				{foo}.into_node_bundle(),
				EventObserver::new("onclick"),
				EntityObserver::new(#[allow(unused_braces)] |_: Trigger<OnClick>| { println!("clicked"); }),
				related!(Attributes[
					"hidden".into_attr_key_bundle(),
					(
						"class".into_attr_key_bundle(),
						"foo".into_attr_val_bundle()
					),
					(
						"onmousemove".into_attr_key_bundle(),
						"some_js_func".into_attr_val_bundle()
					)
				])
			)}
			.to_string(),
		);
	}
}
