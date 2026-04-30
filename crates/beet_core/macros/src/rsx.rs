//! RSX macro for tokenizing JSX-like syntax into Bevy ECS bundles.
//!
//! Converts rstml-parsed HTML-like syntax directly into bundle expressions:
//! - Lowercase tags become `Element::new("tag")`
//! - Capitalized tags become `TagName::default().with_field(val)...`
//! - Attributes become `related!(Attributes[...])` entries
//! - Children become `children![...]` entries
//! - Block expressions `{(foo, bar)}` are spread as tuple components
//! - `{Foo}` on both uppercase and lowercase tags inserts `Foo` as an
//!   additional component
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::pkg_ext;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use rstml::Parser;
use rstml::ParserConfig;
use rstml::node::KeyedAttributeValue;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use syn::spanned::Spanned;

/// Custom node type, currently unused.
type CustomNode = rstml::Infallible;

/// Entry point called from the proc macro.
pub fn impl_rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let parser = Parser::new(
		ParserConfig::new()
			.recover_block(true)
			.macro_call_pattern(quote!(rsx! {%%})),
	);

	let (nodes, errors) = parser
		.parse_recoverable(proc_macro2::TokenStream::from(input))
		.split_vec();
	let error_tokens: Vec<TokenStream> = errors
		.into_iter()
		.map(|err| err.emit_as_expr_tokens())
		.collect();

	let body = if nodes.len() == 1 {
		// single root element, emit directly
		tokenize_node(&nodes[0])
	} else {
		// multiple root elements, wrap in children!
		let items: Vec<TokenStream> = nodes.iter().map(tokenize_node).collect();
		quote! { children![#(#items),*] }
	};
	let beet_node = pkg_ext::internal_or_beet("beet_node");

	let output = quote! {{
		use #beet_node::prelude::*;
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
		Node::RawText(text) => {
			let value = text.to_string_best();
			quote! { Value::new(#value) }
		}
		Node::Block(NodeBlock::ValidBlock(block)) => {
			// block expression in child position, ie `>{expr}<`
			quote! { (#block).into_bundle() }
		}
		Node::Block(NodeBlock::Invalid(invalid)) => {
			let span = invalid.span();
			let err = syn::Error::new(span, "invalid block expression");
			err.into_compile_error()
		}
		Node::Comment(comment) => {
			let value = comment.value.value();
			quote! { Comment::new(#value) }
		}
		Node::Doctype(doctype) => {
			let value = doctype.value.to_string_best();
			quote! { Doctype::new(#value) }
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
		Node::Custom(_) => {
			let err = syn::Error::new(
				Span::call_site(),
				"unhandled custom rstml node",
			);
			err.into_compile_error()
		}
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
fn tokenize_html_element(
	el: &NodeElement<CustomNode>,
	tag: &str,
) -> TokenStream {
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
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				let span = invalid.span();
				let err =
					syn::Error::new(span, "invalid block in element attribute");
				block_attrs.push(err.into_compile_error());
			}
			NodeAttribute::Attribute(attr) => {
				let key_str = attr.key.to_string();
				match &attr.possible_value {
					KeyedAttributeValue::Value(value) => {
						// key=value attribute
						let val_expr = &value.value;
						attr_entries.push(quote! {
							(Attribute::new(#key_str), #val_expr.into_bundle())
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
/// - `{Foo}` block attrs insert `Foo` as an additional component
/// - `{field: val, ...}` blocks become struct init with `..Default::default()`
/// - Flag attributes become `.with_field(true)`
/// - `key=value` attributes become `.with_key(value)`
fn tokenize_component(el: &NodeElement<CustomNode>, tag: &str) -> TokenStream {
	let tag_span = el.open_tag.name.span();
	let tag_ident: syn::Path = match syn::parse_str(tag) {
		Ok(path) => path,
		Err(_) => {
			let err = syn::Error::new(
				tag_span,
				format!("invalid component path: `{tag}`"),
			);
			return err.into_compile_error();
		}
	};

	let mut with_calls: Vec<TokenStream> = Vec::new();
	let mut block_attrs: Vec<TokenStream> = Vec::new();

	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// block attribute inserts as additional component,
				// ie `<MyComponent {Foo}>` inserts Foo alongside MyComponent
				block_attrs.push(quote! { #block });
			}
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				let span = invalid.span();
				let err =
					syn::Error::new(span, "invalid block in element attribute");
				block_attrs.push(err.into_compile_error());
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

	// collect all parts: constructor + with_calls, block attrs, children
	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(quote! { #tag_ident::default() #(#with_calls)*.into_bundle() });

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
