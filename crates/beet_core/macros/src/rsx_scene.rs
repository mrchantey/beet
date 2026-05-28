//! Scene RSX lowering: tokenizes the same JSX-like syntax as
//! [`crate::rsx_direct`], but produces an `impl Scene` that flows through
//! Bevy's `bevy_scene` resolve→build→spawn pipeline.
//!
//! - Lowercase tags become an `Element` template value
//! - Text / `{expr}` become `Value` / child scenes (`{expr}` must be a `Scene`)
//! - Children attach via `RelatedScenes::<ChildOf, _>`
//! - Attributes attach via `RelatedScenes::<AttributeOf, _>`
//! - Event attributes (`on*`) become `on(...)` observer templates
//! - Capitalized tags `<Foo prop=x/>` become `Foo(FooProps::default().with_prop(x))`
//!
//! The consuming crate must enable the `scene` feature, which provides
//! `template_value`, `RelatedScenes`, `EntityScene`, `on`, etc. via its prelude.
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

/// Entry point for the scene-producing variant.
pub fn impl_rsx_scene(
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
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

	// A `Scene` describes a single root entity. A single root node maps
	// directly; multiple roots are not a single scene.
	let body = match nodes.as_slice() {
		[node] => tokenize_node_scene(node),
		[] => quote! { () },
		_ => syn::Error::new(
			Span::call_site(),
			"rsx! expects a single root node (a Scene is one root entity)",
		)
		.into_compile_error(),
	};
	let beet_ui = pkg_ext::internal_or_beet("beet_ui");

	let output = quote! {{
		use #beet_ui::prelude::*;
		#(#error_tokens)*
		#body
	}};
	output.into()
}

/// Tokenize a single rstml node into an `impl Scene` expression.
fn tokenize_node_scene(node: &Node<CustomNode>) -> TokenStream {
	match node {
		Node::Element(el) => tokenize_element_scene(el),
		Node::Text(text) => {
			let value = text.value_string();
			quote! { template_value(Value::new(#value)) }
		}
		Node::RawText(text) => {
			let value = text.to_string_best();
			quote! { template_value(Value::new(#value)) }
		}
		Node::Block(NodeBlock::ValidBlock(block)) => {
			// block expression in child position must itself be an `impl Scene`
			quote! { #block }
		}
		Node::Block(NodeBlock::Invalid(invalid)) => {
			syn::Error::new(invalid.span(), "invalid block expression")
				.into_compile_error()
		}
		Node::Fragment(fragment) => match fragment.children.as_slice() {
			[child] => tokenize_node_scene(child),
			_ => syn::Error::new(
				Span::call_site(),
				"rsx! fragments must contain a single root node",
			)
			.into_compile_error(),
		},
		other => syn::Error::new(
			other.span(),
			"this node kind is not yet supported by rsx!",
		)
		.into_compile_error(),
	}
}

/// Tokenize an element into a scene, dispatching on tag casing.
fn tokenize_element_scene(el: &NodeElement<CustomNode>) -> TokenStream {
	let tag_str = el.open_tag.name.to_string();
	if tag_str.starts_with(|ch: char| ch.is_uppercase()) {
		tokenize_component_scene(el, &tag_str)
	} else {
		tokenize_html_element_scene(el, &tag_str)
	}
}

/// Lower a capitalized component tag `<Foo a=1 b/>` to a `#[scene]` call:
/// `Foo(FooProps::default().with_a(1).with_b(true))`. Block attributes spread
/// extra scenes onto the same entity; children attach via `ChildOf`.
fn tokenize_component_scene(
	el: &NodeElement<CustomNode>,
	tag: &str,
) -> TokenStream {
	let tag_span = el.open_tag.name.span();
	let tag_path: syn::Path = match syn::parse_str(tag) {
		Ok(path) => path,
		Err(_) => {
			return syn::Error::new(
				tag_span,
				format!("invalid component path: `{tag}`"),
			)
			.into_compile_error();
		}
	};
	// the props struct lives next to the function as `{Tag}Props`
	let mut props_path = tag_path.clone();
	if let Some(last) = props_path.segments.last_mut() {
		last.ident = syn::Ident::new(
			&format!("{}Props", last.ident),
			last.ident.span(),
		);
		last.arguments = syn::PathArguments::None;
	}

	let mut with_calls: Vec<TokenStream> = Vec::new();
	let mut block_parts: Vec<TokenStream> = Vec::new();
	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Attribute(attr) => {
				let key_str = attr.key.to_string();
				let setter = syn::Ident::new(
					&format!("with_{key_str}"),
					Span::call_site(),
				);
				match &attr.possible_value {
					KeyedAttributeValue::Value(value) => {
						let val_expr = &value.value;
						with_calls.push(quote! { .#setter(#val_expr) });
					}
					_ => with_calls.push(quote! { .#setter(true) }),
				}
			}
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// block attribute spread is an extra `impl Scene` on this entity
				block_parts.push(quote! { #block });
			}
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				block_parts.push(
					syn::Error::new(
						invalid.span(),
						"invalid block in element attribute",
					)
					.into_compile_error(),
				);
			}
		}
	}

	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(quote! { #tag_path(#props_path::default() #(#with_calls)*) });
	parts.extend(block_parts);

	let child_scenes: Vec<TokenStream> = el
		.children
		.iter()
		.map(|child| {
			let scene = tokenize_node_scene(child);
			quote! { EntityScene(#scene) }
		})
		.collect();
	if !child_scenes.is_empty() {
		parts.push(quote! {
			RelatedScenes::<ChildOf, _>::new((#(#child_scenes,)*))
		});
	}

	quote! { (#(#parts),*) }
}

/// Lower a lowercase HTML element to a scene:
/// - the tag itself becomes an `Element` template value,
/// - attributes become child entities related via `AttributeOf`,
/// - children become entities related via `ChildOf`.
fn tokenize_html_element_scene(
	el: &NodeElement<CustomNode>,
	tag: &str,
) -> TokenStream {
	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(quote! { template_value(Element::new(#tag)) });

	// attributes -> `RelatedScenes::<AttributeOf, _>`; event attributes
	// (`on*`) instead become observer templates on this entity.
	let mut attr_scenes: Vec<TokenStream> = Vec::new();
	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Attribute(attr) => {
				let key_str = attr.key.to_string();
				let value = match &attr.possible_value {
					KeyedAttributeValue::Value(value) => Some(&value.value),
					_ => None,
				};
				match (key_str.starts_with("on"), value) {
					(true, Some(val_expr)) => {
						// event attribute -> observer (`on(...)` is a Scene)
						parts.push(quote! { on(#val_expr) });
					}
					(true, None) => parts.push(
						syn::Error::new(
							attr.key.span(),
							"event attribute requires a handler value",
						)
						.into_compile_error(),
					),
					(false, Some(val_expr)) => attr_scenes.push(quote! {
						EntityScene((
							template_value(Attribute::new(#key_str)),
							template_value(Value::new(#val_expr)),
						))
					}),
					(false, None) => {
						// flag attribute: `Value` comes from `#[require(Value)]`
						attr_scenes.push(quote! {
							EntityScene(template_value(Attribute::new(#key_str)))
						});
					}
				}
			}
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// block attribute spread is itself an `impl Scene` on this entity
				parts.push(quote! { #block });
			}
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				parts.push(
					syn::Error::new(
						invalid.span(),
						"invalid block in element attribute",
					)
					.into_compile_error(),
				);
			}
		}
	}
	if !attr_scenes.is_empty() {
		parts.push(quote! {
			RelatedScenes::<AttributeOf, _>::new((#(#attr_scenes,)*))
		});
	}

	// children -> `RelatedScenes::<ChildOf, _>`
	let child_scenes: Vec<TokenStream> = el
		.children
		.iter()
		.map(|child| {
			let scene = tokenize_node_scene(child);
			quote! { EntityScene(#scene) }
		})
		.collect();
	if !child_scenes.is_empty() {
		parts.push(quote! {
			RelatedScenes::<ChildOf, _>::new((#(#child_scenes,)*))
		});
	}

	quote! { (#(#parts),*) }
}
