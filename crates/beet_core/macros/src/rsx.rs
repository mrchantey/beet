//! RSX macro for tokenizing JSX-like syntax into Bevy ECS bundles.
//!
//! Converts rstml-parsed HTML-like syntax directly into bundle expressions:
//! - Lowercase tags become `Element::new("tag")`
//! - Capitalized tags become `TagName::default().with_field(val)...`
//! - Attributes become `related!(Attributes[...])` entries
//! - Children become `children![...]` entries
//! - Block expressions `{(foo, bar)}` are spread as tuple components
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;
use rstml::node::KeyedAttributeValue;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::Parser;
use rstml::ParserConfig;

/// Custom node type, currently unused.
type CustomNode = rstml::Infallible;

/// Entry point called from the proc macro.
pub fn impl_rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let parser = Parser::new(
		ParserConfig::new()
			.recover_block(true)
			.macro_call_pattern(quote!(rsx! {%%})),
	);

	let (nodes, errors) = parser.parse_recoverable(proc_macro2::TokenStream::from(input)).split_vec();
	let error_tokens: Vec<TokenStream> = errors
		.into_iter()
		.map(|err| err.emit_as_expr_tokens())
		.collect();

	let body = if nodes.len() == 1 {
		// single root element, emit directly
		tokenize_node(&nodes[0])
	} else {
		// multiple root elements, wrap in children!
		let items: Vec<TokenStream> =
			nodes.iter().map(tokenize_node).collect();
		quote! { children![#(#items),*] }
	};

	let output = quote! {{
		use beet_node::prelude::*;
		#(#error_tokens)*
		#body
	}};
	output.into()
}

/// Tokenize a single rstml node into a bundle expression.
fn tokenize_node(node: &Node<CustomNode>) -> TokenStream {
	match node {
		Node::Element(el) => tokenize_element(el),
		Node::Text(text) => {
			let value = text.value_string();
			quote! { Value::new(#value) }
		}
		Node::Block(NodeBlock::ValidBlock(block)) => {
			// block expression in child position, ie `>{expr}<`
			quote! { (#block).into_bundle() }
		}
		Node::Comment(comment) => {
			let value = comment.value.value();
			quote! { Comment::new(#value) }
		}
		Node::Fragment(fragment) => {
			let items: Vec<TokenStream> =
				fragment.children.iter().map(tokenize_node).collect();
			if items.len() == 1 {
				items.into_iter().next().unwrap()
			} else {
				quote! { children![#(#items),*] }
			}
		}
		_ => quote! { () },
	}
}

/// Tokenize an element node. Dispatches based on whether the tag
/// starts with an uppercase letter (component) or lowercase (HTML element).
fn tokenize_element(el: &NodeElement<CustomNode>) -> TokenStream {
	let tag_str = el.open_tag.name.to_string();
	let is_component = tag_str.starts_with(|ch: char| ch.is_uppercase());

	if is_component {
		tokenize_component(el, &tag_str)
	} else {
		tokenize_html_element(el, &tag_str)
	}
}

/// Tokenize a lowercase HTML element like `<div foo="bar">child</div>`.
fn tokenize_html_element(el: &NodeElement<CustomNode>, tag: &str) -> TokenStream {
	let mut parts: Vec<TokenStream> = Vec::new();

	// element tag
	parts.push(quote! { Element::new(#tag) });

	// collect attributes
	let mut attr_entries: Vec<TokenStream> = Vec::new();
	let mut block_attrs: Vec<TokenStream> = Vec::new();

	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// block attribute spread, ie `<div {(foo, bar)}>`
				block_attrs.push(quote! { #block });
			}
			NodeAttribute::Attribute(attr) => {
				let key_str = attr.key.to_string();
				match &attr.possible_value {
					KeyedAttributeValue::Value(value) => {
						// key=value attribute
						let val_expr = &value.value;
						attr_entries.push(quote! {
							(Attribute::new(#key_str), Value::new(#val_expr))
						});
					}
					_ => {
						// flag attribute, ie `<div hidden>`
						attr_entries.push(quote! {
							Attribute::new(#key_str)
						});
					}
				}
			}
			_ => {}
		}
	}

	if !attr_entries.is_empty() {
		parts.push(quote! {
			related!(Attributes[#(#attr_entries),*])
		});
	}

	// block attribute spreads become direct tuple members
	for block in block_attrs {
		parts.push(quote! { #block });
	}

	// children
	let child_tokens: Vec<TokenStream> =
		el.children.iter().map(tokenize_node).collect();
	if !child_tokens.is_empty() {
		parts.push(quote! {
			children![#(#child_tokens),*]
		});
	}

	// wrap in tuple
	quote! { (#(#parts),*) }
}

/// Tokenize a capitalized component like `<MyComponent foo bar=bazz />`.
///
/// - `{field: val, ...}` blocks become struct init with `..Default::default()`
/// - Flag attributes become `.with_field(true)`
/// - `key=value` attributes become `.with_key(value)`
fn tokenize_component(el: &NodeElement<CustomNode>, tag: &str) -> TokenStream {
	let tag_ident: syn::Path =
		syn::parse_str(tag).expect("invalid component path");

	let mut with_calls: Vec<TokenStream> = Vec::new();
	let mut struct_fields: Option<TokenStream> = None;

	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// valid block attribute, ie `<MyComponent {valid_expr}>`
				struct_fields = Some(quote! { #block });
			}
			NodeAttribute::Block(NodeBlock::Invalid(tokens)) => {
				// struct init block, ie `<MyComponent {foo: bar, bazz: boo}>`
				// these are not valid Rust expressions so rstml puts them in Invalid
				struct_fields = Some(quote! { { #tokens, ..Default::default() } });
			}
			NodeAttribute::Attribute(attr) => {
				let key_str = attr.key.to_string();
				let key_ident = syn::Ident::new(
					&format!("with_{}", key_str),
					proc_macro2::Span::call_site(),
				);
				match &attr.possible_value {
					KeyedAttributeValue::Value(value) => {
						let val_expr = &value.value;
						with_calls.push(quote! { .#key_ident(#val_expr) });
					}
					_ => {
						// flag attribute -> .with_field(true)
						with_calls.push(quote! { .#key_ident(true) });
					}
				}
			}
		}
	}

	let constructor = if let Some(fields) = struct_fields {
		// struct init with provided fields and ..Default::default()
		quote! { #tag_ident #fields }
	} else {
		quote! { <#tag_ident as Default>::default() }
	};

	let mut expr = quote! { #constructor #(#with_calls)* };

	// children of component elements
	let child_tokens: Vec<TokenStream> =
		el.children.iter().map(tokenize_node).collect();
	if !child_tokens.is_empty() {
		expr = quote! {
			(#expr.into_bundle(), children![#(#child_tokens),*])
		};
	} else {
		expr = quote! { #expr.into_bundle() };
	}

	expr
}
