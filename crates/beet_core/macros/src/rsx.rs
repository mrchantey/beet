//! The single `rsx!` markup lowering.
//!
//! Lowers JSX-like markup to a [`Bundle`] tree on the
//! `Element`/`Attribute`/`children!`/`Value` base, wrapped at the root by
//! `snippet(..)` into the `impl Template<Output = ()>` the substrate's
//! `spawn_template` accepts. This merges the former `rsx!`/`rsx_direct!` pair
//! into one macro targeting the template substrate (no `bevy_scene`).
//!
//! Lowering rules (see `.agents/plans/bsx/macros.md`):
//!
//! - A lowercase tag becomes an [`Element`] with attribute children.
//! - An uppercase tag is a **component** (reflect-patched, inserted) or a
//!   **template** (built with input props). The macro cannot know which, so it
//!   emits a static struct update `Foo { a: a.into(), ..Default::default() }`
//!   dispatched at runtime through `IntoSnippetBundle` (a `Component` inserts, a
//!   build-subtree `Template` builds).
//! - A bare `{..}` in attribute position is a **spread**: components/templates
//!   inserted onto the entity, also through `IntoSnippetBundle`.
//! - An attribute or text `{..}` is a **value**: a Rust expression, lifted via
//!   `IntoSnippet`.
//! - `<Slot/>` lowers to a `SlotTarget` marker; `bx:slot="x"`/`slot="x"` on a
//!   node lowers to a `SlotChild` marker. The walker resolves them.
//! - `on*` / `bx:click` events lower to observers.
//!
//! The lowering entry point is [`lower_rsx`], a plain function `#[template]`
//! calls in process (emitting an `rsx!` macro *call* from inside the attribute
//! macro makes rstml's macro-call recovery panic, see `macros.md`).
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
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

/// The `rsx!` proc-macro entry: lowers markup to an `impl Template<Output = ()>`
/// by wrapping the lowered bundle in `snippet(..)`.
pub fn impl_rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let bundle = lower_rsx(input.into());
	quote! {{
		use #beet_core::prelude::*;
		snippet(#bundle)
	}}
	.into()
}

/// Lower markup token stream to a [`Bundle`] expression.
///
/// This is the in-process entry point both the `rsx!` macro (which wraps the
/// result in `snippet(..)`) and `#[template]` (which uses it as the function
/// body's `impl Bundle`) call. It performs no `use` injection; the caller scopes
/// the emitted identifiers (`Element`, `Value`, `IntoSnippet`, …) via
/// `use beet_core::prelude::*`.
pub fn lower_rsx(input: TokenStream) -> TokenStream {
	// No `macro_call_pattern`: this function is the single lowering pass, called
	// in process by both `rsx!` and `#[template]`. The pattern triggers rstml's
	// macro-call recovery, which crashes on macro-generated tokens ("Cannot find
	// macro pattern inside Span::call_site"); see `.agents/plans/bsx/macros.md`.
	let parser = Parser::new(ParserConfig::new().recover_block(true));
	let (nodes, errors) = parser.parse_recoverable(input).split_vec();
	let error_tokens: Vec<TokenStream> = errors
		.into_iter()
		.map(|err| err.emit_as_expr_tokens())
		.collect();

	let body = match nodes.len() {
		// a single root node lowers directly.
		1 => tokenize_node(&nodes[0]),
		// no nodes: an empty bundle.
		0 => quote! { () },
		// multiple roots: a fragment spawning each as a child entity.
		_ => {
			let items: Vec<TokenStream> =
				nodes.iter().map(tokenize_node).collect();
			quote! { children![#(#items),*] }
		}
	};

	quote! {{
		#(#error_tokens)*
		#body
	}}
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
			// text-position `{expr}`: a value lifted via `IntoSnippet`.
			quote! { (#block).into_snippet() }
		}
		Node::Block(NodeBlock::Invalid(invalid)) => {
			syn::Error::new(invalid.span(), "invalid block expression")
				.into_compile_error()
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
			match items.len() {
				1 => items.into_iter().next().unwrap(),
				_ => quote! { children![#(#items),*] },
			}
		}
		Node::Custom(_) => {
			syn::Error::new(Span::call_site(), "unhandled custom rstml node")
				.into_compile_error()
		}
	}
}

/// Tokenize an element, dispatching on tag kind: `<Slot>` -> slot target,
/// uppercase -> component/template, lowercase -> HTML element.
fn tokenize_element(el: &NodeElement<CustomNode>) -> TokenStream {
	let tag_str = el.open_tag.name.to_string();
	if tag_str == "Slot" {
		return tokenize_slot(el);
	}
	if tag_str.starts_with(|ch: char| ch.is_uppercase()) {
		tokenize_component(el, &tag_str)
	} else {
		tokenize_html_element(el, &tag_str)
	}
}

/// Lower a `<Slot/>` placeholder to a `SlotTarget` marker. `<Slot name="x"/>`
/// targets the named slot; children are the fallback rendered when the caller
/// supplies no matching content (the walker drops them otherwise).
///
/// An Astro-style transfer `<Slot name="x" bx:slot="y"/>` (or `slot="y"`)
/// forwards content received in this template's `x` slot into the `y` slot of
/// the enclosing template: it lowers to both a `SlotTarget` and a `SlotChild`,
/// which the walker re-opens as it composes.
fn tokenize_slot(el: &NodeElement<CustomNode>) -> TokenStream {
	let target = match slot_name_attr(el) {
		Some(name) => quote! { SlotTarget::named(#name) },
		None => quote! { SlotTarget::new() },
	};
	let mut parts: Vec<TokenStream> = vec![target];
	// a transfer also carries a `SlotChild` routing it into a parent slot.
	if let Some(transfer) = el.open_tag.attributes.iter().find_map(|attr| {
		let NodeAttribute::Attribute(attr) = attr else {
			return None;
		};
		let key = attr.key.to_string();
		(key == "slot" || key == "bx:slot").then(|| slot_child_marker(attr))
	}) {
		parts.push(transfer);
	}
	let fallback: Vec<TokenStream> =
		el.children.iter().map(tokenize_node).collect();
	if !fallback.is_empty() {
		parts.push(quote! { children![#(#fallback),*] });
	}
	match parts.len() {
		1 => parts.into_iter().next().unwrap(),
		_ => quote! { (#(#parts),*) },
	}
}

/// Read a `<Slot name="x"/>`'s `name` attribute (string literals only).
fn slot_name_attr(el: &NodeElement<CustomNode>) -> Option<String> {
	el.open_tag.attributes.iter().find_map(|attr| {
		let NodeAttribute::Attribute(attr) = attr else {
			return None;
		};
		if attr.key.to_string() != "name" {
			return None;
		}
		match &attr.possible_value {
			KeyedAttributeValue::Value(value) => value.value_literal_string(),
			_ => None,
		}
	})
}

/// Extract a node's `slot="name"` / `bx:slot="name"` routing attribute, marking
/// content destined for a parent template's named slot. Default-slot content has
/// no such attribute.
fn node_slot_child(node: &Node<CustomNode>) -> Option<TokenStream> {
	let Node::Element(el) = node else {
		return None;
	};
	el.open_tag.attributes.iter().find_map(|attr| {
		let NodeAttribute::Attribute(attr) = attr else {
			return None;
		};
		let key = attr.key.to_string();
		if key != "slot" && key != "bx:slot" {
			return None;
		}
		match &attr.possible_value {
			KeyedAttributeValue::Value(value) => value
				.value_literal_string()
				.map(|name| quote! { SlotChild::named(#name) }),
			_ => Some(quote! { SlotChild::new() }),
		}
	})
}

/// Tokenize a lowercase HTML element like `<div foo="bar">child</div>`.
fn tokenize_html_element(
	el: &NodeElement<CustomNode>,
	tag: &str,
) -> TokenStream {
	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(quote! { Element::new(#tag) });

	let mut attr_entries: Vec<TokenStream> = Vec::new();
	let mut extra: Vec<TokenStream> = Vec::new();

	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				// bare `{..}` spread: components/templates onto this entity.
				extra.push(quote! { (#block).into_snippet_bundle() });
			}
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				extra.push(
					syn::Error::new(
						invalid.span(),
						"invalid block in element attribute",
					)
					.into_compile_error(),
				);
			}
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				// `slot`/`bx:slot` route this element into a parent's named slot;
				// emitted as a `SlotChild` marker, not an HTML attribute.
				if key == "slot" || key == "bx:slot" {
					extra.push(slot_child_marker(attr));
					continue;
				}
				let value = match &attr.possible_value {
					KeyedAttributeValue::Value(value) => Some(&value.value),
					_ => None,
				};
				match (key.starts_with("on"), value) {
					(true, Some(val)) => {
						// `on*` event handler -> observer.
						extra.push(quote! { (#val).into_snippet() });
					}
					(true, None) => extra.push(
						syn::Error::new(
							attr.key.span(),
							"event attribute requires a handler value",
						)
						.into_compile_error(),
					),
					(false, Some(val)) => attr_entries.push(quote! {
						(Attribute::new(#key), Value::new(#val))
					}),
					(false, None) => attr_entries.push(quote! {
						Attribute::new(#key)
					}),
				}
			}
		}
	}

	if !attr_entries.is_empty() {
		parts.push(quote! { related!(Attributes[#(#attr_entries),*]) });
	}
	parts.extend(extra);

	let child_tokens: Vec<TokenStream> =
		el.children.iter().map(tokenize_node).collect();
	if !child_tokens.is_empty() {
		parts.push(quote! { children![#(#child_tokens),*] });
	}

	quote! { (#(#parts),*) }
}

/// Emit the `SlotChild` marker for a `slot`/`bx:slot` routing attribute.
fn slot_child_marker(attr: &rstml::node::KeyedAttribute) -> TokenStream {
	match &attr.possible_value {
		KeyedAttributeValue::Value(value) => match value.value_literal_string()
		{
			Some(name) => quote! { SlotChild::named(#name) },
			None => quote! { SlotChild::new() },
		},
		_ => quote! { SlotChild::new() },
	}
}

/// Tokenize a capitalized tag `<Foo a=x b/>` to a component patch / template
/// build, dispatched at runtime by `IntoSnippetBundle`.
///
/// Values lower to a static struct update `Foo { a: x.into(), b: true.into(),
/// ..Default::default() }`. Caller content becomes
/// children carrying `SlotChild` markers, matched to the template's
/// `SlotTarget`s by the walker. Bare `{..}` attrs spread extra
/// components/templates onto the same entity.
fn tokenize_component(el: &NodeElement<CustomNode>, tag: &str) -> TokenStream {
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

	let mut fields: Vec<TokenStream> = Vec::new();
	let mut spreads: Vec<TokenStream> = Vec::new();
	let mut self_slot: Option<TokenStream> = None;

	for attr in &el.open_tag.attributes {
		match attr {
			NodeAttribute::Attribute(attr) => {
				let key = attr.key.to_string();
				// a `slot`/`bx:slot` on the component tag routes the whole built
				// component into a parent's named slot.
				if key == "slot" || key == "bx:slot" {
					self_slot = Some(slot_child_marker(attr));
					continue;
				}
				let field = syn::Ident::new(&key, attr.key.span());
				match &attr.possible_value {
					KeyedAttributeValue::Value(value) => {
						let val = &value.value;
						fields.push(quote! { #field: (#val).into_prop() });
					}
					// flag attribute -> `field: true.into_prop()`.
					_ => fields.push(quote! { #field: (true).into_prop() }),
				}
			}
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				spreads.push(quote! { (#block).into_snippet_bundle() });
			}
			NodeAttribute::Block(NodeBlock::Invalid(invalid)) => {
				spreads.push(
					syn::Error::new(
						invalid.span(),
						"invalid block in element attribute",
					)
					.into_compile_error(),
				);
			}
		}
	}

	// the patch-over-default struct update, dispatched insert-vs-build.
	let patch = quote! {
		(#tag_path { #(#fields,)* ..Default::default() }).into_snippet_bundle()
	};

	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(patch);
	parts.extend(spreads);

	// caller content: each child spawns its own entity carrying a `SlotChild`
	// marker, routed by the walker. A child with its own `slot=`/`bx:slot=`
	// already emits a named `SlotChild` via its element lowering; an unmarked
	// child goes to the default slot, so wrap it here.
	let child_tokens: Vec<TokenStream> = el
		.children
		.iter()
		.map(|child| {
			let inner = tokenize_node(child);
			if node_slot_child(child).is_some() {
				inner
			} else {
				quote! { (#inner, SlotChild::new()) }
			}
		})
		.collect();
	if !child_tokens.is_empty() {
		parts.push(quote! { children![#(#child_tokens),*] });
	}

	let built = quote! { (#(#parts),*) };
	// if the tag itself routes into a parent's slot, carry that marker too.
	match self_slot {
		Some(marker) => quote! { (#built, #marker) },
		None => built,
	}
}
