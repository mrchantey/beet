//! Lowering from the [`ast`](super::ast) to a [`Bundle`] token stream.
//!
//! Lowering rules:
//!
//! - A lowercase tag becomes an `Element` with attribute children.
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
use super::ast::*;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use proc_macro2::TokenStream;
use quote::quote;

/// Lower a list of root nodes to a single [`Bundle`] expression: one node
/// directly, none to an empty bundle, many to a fragment spawning each child.
pub fn lower_nodes(nodes: &[RsxNode]) -> TokenStream {
	match nodes.len() {
		1 => tokenize_node(&nodes[0]),
		0 => quote! { () },
		_ => {
			let items = nodes.iter().map(tokenize_node);
			quote! { children![#(#items),*] }
		}
	}
}

/// Tokenize a single node into a bundle expression.
fn tokenize_node(node: &RsxNode) -> TokenStream {
	match node {
		RsxNode::Element(el) => tokenize_element(el),
		RsxNode::Text(text) => {
			let value = text.value();
			quote! { Value::new(#value) }
		}
		RsxNode::Block(block) => {
			// text-position `{expr}`: a value lifted via `IntoSnippet`.
			quote! { (#block).into_snippet() }
		}
		RsxNode::Comment(value) => quote! { Comment::new(#value) },
		RsxNode::Doctype(value) => quote! { Doctype::new(#value) },
		RsxNode::Fragment(fragment) => {
			let items: Vec<TokenStream> =
				fragment.children.iter().map(tokenize_node).collect();
			match items.len() {
				1 => items.into_iter().next().unwrap(),
				_ => quote! { children![#(#items),*] },
			}
		}
	}
}

/// Tokenize an element, dispatching on tag kind: `<Slot>` -> slot target,
/// uppercase -> component/template, lowercase -> HTML element.
fn tokenize_element(el: &RsxElement) -> TokenStream {
	let tag = el.name.value();
	if tag == "Slot" {
		tokenize_slot(el)
	} else if tag.starts_with(|ch: char| ch.is_uppercase()) {
		tokenize_component(el, &tag)
	} else {
		tokenize_html_element(el, &tag)
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
fn tokenize_slot(el: &RsxElement) -> TokenStream {
	let target = match slot_name_attr(el) {
		Some(name) => quote! { SlotTarget::named(#name) },
		None => quote! { SlotTarget::new() },
	};
	let mut parts: Vec<TokenStream> = vec![target];
	// a transfer also carries a `SlotChild` routing it into a parent slot.
	if let Some(transfer) = el.attributes.iter().find_map(|attr| {
		let RsxAttr::Keyed(attr) = attr else {
			return None;
		};
		let key = attr.key_str();
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
fn slot_name_attr(el: &RsxElement) -> Option<alloc::string::String> {
	el.attributes.iter().find_map(|attr| {
		let RsxAttr::Keyed(attr) = attr else {
			return None;
		};
		(attr.key_str() == "name")
			.then(|| attr.value_literal_string())
			.flatten()
	})
}

/// Extract a node's `slot="name"` / `bx:slot="name"` routing attribute, marking
/// content destined for a parent template's named slot. Default-slot content has
/// no such attribute.
fn node_slot_child(node: &RsxNode) -> Option<TokenStream> {
	let RsxNode::Element(el) = node else {
		return None;
	};
	el.attributes.iter().find_map(|attr| {
		let RsxAttr::Keyed(attr) = attr else {
			return None;
		};
		let key = attr.key_str();
		if key != "slot" && key != "bx:slot" {
			return None;
		}
		Some(slot_child_marker(attr))
	})
}

/// Tokenize a lowercase HTML element like `<div foo="bar">child</div>`.
fn tokenize_html_element(el: &RsxElement, tag: &str) -> TokenStream {
	let mut parts: Vec<TokenStream> = Vec::new();
	parts.push(quote! { Element::new(#tag) });

	let mut attr_entries: Vec<TokenStream> = Vec::new();
	let mut extra: Vec<TokenStream> = Vec::new();

	for attr in &el.attributes {
		match attr {
			RsxAttr::Spread(block) => {
				// bare `{..}` spread: components/templates onto this entity.
				extra.push(quote! { (#block).into_snippet_bundle() });
			}
			RsxAttr::Keyed(attr) => {
				let key = attr.key_str();
				// `slot`/`bx:slot` route this element into a parent's named slot;
				// emitted as a `SlotChild` marker, not an HTML attribute.
				if key == "slot" || key == "bx:slot" {
					extra.push(slot_child_marker(attr));
					continue;
				}
				let value = attr.value_expr();
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
fn slot_child_marker(attr: &RsxKeyedAttr) -> TokenStream {
	match attr.value_literal_string() {
		Some(name) => quote! { SlotChild::named(#name) },
		None => quote! { SlotChild::new() },
	}
}

/// Tokenize a capitalized tag `<Foo a=x b/>` to a component patch / template
/// build, dispatched at runtime by `IntoSnippetBundle`.
///
/// Values lower to a static struct update `Foo { a: x.into(), b: true.into(),
/// ..Default::default() }`. Caller content becomes children carrying `SlotChild`
/// markers, matched to the template's `SlotTarget`s by the walker. Bare `{..}`
/// attrs spread extra components/templates onto the same entity.
fn tokenize_component(el: &RsxElement, tag: &str) -> TokenStream {
	let tag_span = el.name.span();
	let Some(tag_path) = el.name.as_path() else {
		return syn::Error::new(
			tag_span,
			format!("invalid component path: `{tag}`"),
		)
		.into_compile_error();
	};

	let mut fields: Vec<TokenStream> = Vec::new();
	let mut spreads: Vec<TokenStream> = Vec::new();
	let mut self_slot: Option<TokenStream> = None;

	for attr in &el.attributes {
		match attr {
			RsxAttr::Keyed(attr) => {
				let key = attr.key_str();
				// a `slot`/`bx:slot` on the component tag routes the whole built
				// component into a parent's named slot.
				if key == "slot" || key == "bx:slot" {
					self_slot = Some(slot_child_marker(attr));
					continue;
				}
				let field = syn::Ident::new(&key, attr.key.span());
				match attr.value_expr() {
					Some(val) => {
						fields.push(quote! { #field: (#val).into_prop() });
					}
					// flag attribute -> `field: true.into_prop()`.
					None => fields.push(quote! { #field: (true).into_prop() }),
				}
			}
			RsxAttr::Spread(block) => {
				spreads.push(quote! { (#block).into_snippet_bundle() });
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
