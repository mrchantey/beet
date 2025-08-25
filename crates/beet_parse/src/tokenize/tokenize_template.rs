use crate::prelude::NodeExpr;
use crate::tokenize::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;


pub fn tokenize_template(
	world: &World,
	entity_components: &mut Vec<TokenStream>,
	entity: Entity,
) -> Result<()> {
	let entity = world.entity(entity);
	if !entity.contains::<TemplateNode>() {
		return Ok(());
	}

	let Some(node_tag) = entity.get::<NodeTag>() else {
		return Ok(());
	};
	let node_tag_span = entity.get::<SpanOf<NodeTag>>();
	let mut prop_assignments = Vec::new();

	if let Some(attrs) = entity.get::<Attributes>() {
		for attr_entity in attrs.iter() {
			let key = maybe_spanned_attr_key(world, attr_entity).map(
				|(key, span)| {
					let ident = non_reserved_key(&key, span);
					(key, ident)
				},
			);

			let value = world.entity(attr_entity).get::<NodeExpr>().cloned();

			match (key, value) {
				// 1: Events
				(Some((key_str, key)), Some(mut value))
					if is_event(&key_str, &value) =>
				{
					tokenize_event_handler(&key_str, key.span(), &mut value)?;
					let value = value.inner_parsed();
					prop_assignments.push(quote! {.#key(#value)});
				}
				// 2. Key with value
				(Some((_, key)), Some(value)) => {
					let value = value.inner_parsed();
					prop_assignments.push(quote! {.#key(#value)});
				}
				// 3. Key without value (boolean attribute)
				(Some((_, key)), None) => {
					prop_assignments.push(quote! {.#key(true)});
				}
				// 4. Value without key (block/spread attribute)
				(None, Some(value)) => {
					entity_components.push(value.insert_deferred());
				}
				// 5. No key or value, should be unreachable but no big deal
				(None, None) => {}
			}
		}
	}

	let template_ident = Ident::new(
		&node_tag.as_str(),
		node_tag_span.map(|s| **s).unwrap_or(Span::call_site()),
	);

	let template_def = quote! {
			<#template_ident as Props>::Builder::default()
			#(#prop_assignments)*
			.build()
	};

	let inner = if entity.contains::<ClientLoadDirective>()
		|| entity.contains::<ClientOnlyDirective>()
	{
		// this also adds a TemplateRoot::spawn() via component hook using a reflect clone
		syn::parse_quote! {
			ClientIslandRoot::new(#template_def)
		}
	} else {
		syn::parse_quote! {
			TemplateRoot::spawn(Spawn(#template_def.into_bundle()))
		}
	};
	entity_components.push(NodeExpr::new(inner).insert_deferred());

	Ok(())
}

fn non_reserved_key(key: &str, span: Span) -> Ident {
	use bevy::platform::collections::HashSet;
	use std::sync::LazyLock;

	/// All reserved keywords as defined in https://doc.rust-lang.org/reference/keywords.html
	static RESERVED_KEYWORDS: LazyLock<HashSet<&'static str>> =
		LazyLock::new(|| {
			[
				// All editions
				"as",
				"break",
				"const",
				"continue",
				"crate",
				"else",
				"enum",
				"extern",
				"false",
				"fn",
				"for",
				"if",
				"impl",
				"in",
				"let",
				"loop",
				"match",
				"mod",
				"move",
				"mut",
				"pub",
				"ref",
				"return",
				"self",
				"Self",
				"static",
				"struct",
				"super",
				"trait",
				"true",
				"type",
				"unsafe",
				"use",
				"where",
				"while",
				// 2018 edition
				"async",
				"await",
				"dyn",
				// Reserved for future use
				"abstract",
				"become",
				"box",
				"do",
				"final",
				"macro",
				"override",
				"priv",
				"typeof",
				"unsized",
				"virtual",
				"yield",
				// Reserved 2018
				"try",
				// Reserved 2024
				"gen",
				// Weak keywords
				"'static",
				"macro_rules",
				"raw",
				"safe",
				"union",
			]
			.iter()
			.copied()
			.collect()
		});
	if RESERVED_KEYWORDS.contains(&key) {
		Ident::new_raw(&key, span)
	} else if let Some(inner) = key.strip_prefix("r#") {
		// already escaped
		Ident::new_raw(&inner, span)
	} else {
		Ident::new(key, span)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> Matcher<TokenStream> {
		tokenize_rstml(tokens, WsPathBuf::new(file!()))
			.unwrap()
			.xpect()
	}

	#[test]
	fn key_value() {
		quote! {
			<Foo bar client:load/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
	#[test]
	fn reserved_names() {
		quote! {
			<Foo type="bar"/>
		}
		.xmap(parse)
		.to_be_snapshot();
	}
}
